extern crate ra_ap_hir;
extern crate ra_ap_hir_def;
extern crate ra_ap_hir_expand;
extern crate ra_ap_hir_ty;
extern crate ra_ap_syntax;

use std::cell::RefCell;
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::rc::Rc;

use anyhow::{bail, format_err};
use git2::{Delta, Diff, DiffDelta, DiffHunk, Reference};
use upsilon_difftests_core::CoreTestDesc;

pub type DifftestsResult<T = ()> = anyhow::Result<T>;

pub struct Context {
    tempdir: PathBuf,
    core_test_desc: CoreTestDesc,
    git_repo: git2::Repository,

    procdata_file: RefCell<Option<PathBuf>>,
}

pub fn load(tempdir: PathBuf) -> DifftestsResult<Context> {
    load_with_git_repo_from(tempdir.clone(), &tempdir)
}

pub fn load_with_git_repo_from(tempdir: PathBuf, git_repo_path: &Path) -> DifftestsResult<Context> {
    let git_repo = git2::Repository::discover(git_repo_path)?;

    let p = tempdir.join("self.json");

    if !p.exists() {
        bail!("self.json does not exist; did you call `upsilon_difftests_testclient::init`, and did it return `Ok`?");
    }

    if !tempdir.join("self.profraw").exists() {
        bail!("self.profraw does not exist; did the test properly exit?");
    }

    let s = std::fs::read_to_string(p)?;
    let core_test_desc = serde_json::from_str::<CoreTestDesc>(&s)?;

    let context = Context {
        tempdir,
        core_test_desc,
        git_repo,
        procdata_file: RefCell::new(None),
    };

    Ok(context)
}

fn context_procraw_paths(context: &Context) -> DifftestsResult<Vec<PathBuf>> {
    let mut paths = Vec::new();

    for entry in std::fs::read_dir(&context.tempdir)? {
        let entry = entry?;
        let path = entry.path();

        if matches!(path.extension(), Some(s) if s == "profraw") {
            paths.push(path);
        }
    }

    Ok(paths)
}

pub fn context_merge_procraws(
    context: &Context,
    out_file_name: String,
) -> DifftestsResult<PathBuf> {
    let paths = context_procraw_paths(context)?;

    let p = context.tempdir.join(out_file_name);

    let mut cmd = Command::new("rust-profdata");

    cmd.arg("merge")
        .arg("-sparse")
        .args(paths)
        .arg("-o")
        .arg(&p);

    let status = cmd.status()?;

    if !status.success() {
        bail!("rust-profdata failed");
    }

    context.procdata_file.borrow_mut().replace(p.clone());

    Ok(p)
}

pub struct AnalysisContext {
    tempdir: PathBuf,
    core_test_desc: CoreTestDesc,
    git_repo: git2::Repository,
    bins: Vec<PathBuf>,
    procdata: PathBuf,
    ignore_registry_files: bool,
    data_json: PathBuf,
    data: analysis_data::CoverageData,
}

pub mod analysis_data;

pub fn context_export_procdata<F>(
    context: Context,
    bin_from_testdata: F,
    other_bins: &[PathBuf],
    out_file_name: String,
    ignore_registry_files: bool,
) -> DifftestsResult<AnalysisContext>
where
    F: FnOnce(&CoreTestDesc) -> PathBuf,
{
    let data_json = context.tempdir.join(out_file_name);

    let procdata = context
        .procdata_file
        .borrow()
        .as_ref()
        .ok_or_else(|| {
            format_err!(
                "procdata file missing from context; did you call `context_merge_procraws` ?"
            )
        })?
        .clone();

    let mut bins = vec![bin_from_testdata(&context.core_test_desc)];
    bins.extend(other_bins.iter().cloned());

    let mut cmd = Command::new("rust-cov");

    cmd.arg("export")
        .arg("-instr-profile")
        .arg(&procdata)
        .args(&bins)
        .stdout(Stdio::from(File::create(&data_json)?));

    #[cfg(not(windows))]
    const REGISTRY_FILES_REGEX: &str = r"/.cargo/registry";

    #[cfg(windows)]
    const REGISTRY_FILES_REGEX: &str = r#"\\.cargo\\registry"#;

    if ignore_registry_files {
        cmd.arg("--ignore-filename-regex").arg(REGISTRY_FILES_REGEX);
    }

    let status = cmd.status()?;

    if !status.success() {
        bail!("rust-cov failed");
    }

    let Context {
        tempdir,
        core_test_desc,
        git_repo,
        ..
    } = context;

    let data_json_str = std::fs::read_to_string(&data_json)?;
    let data = serde_json::from_str::<analysis_data::CoverageData>(&data_json_str)?;

    let analysis_context = AnalysisContext {
        tempdir,
        core_test_desc,
        git_repo,
        bins,
        procdata,
        ignore_registry_files,
        data_json,
        data,
    };

    Ok(analysis_context)
}

pub fn context_head_worktree_diff(context: &AnalysisContext) -> DifftestsResult<Diff> {
    let head = context.git_repo.head()?;

    context_ref_worktree_diff(context, head)
}

pub fn context_reference_worktree_diff<'ctx>(
    context: &'ctx AnalysisContext,
    reference: &str,
) -> DifftestsResult<Diff<'ctx>> {
    let reference = context.git_repo.find_reference(reference)?;

    context_ref_worktree_diff(context, reference)
}

fn context_ref_worktree_diff<'ctx>(
    context: &'ctx AnalysisContext,
    reference: Reference<'_>,
) -> DifftestsResult<Diff<'ctx>> {
    let reference_tree = reference.peel_to_tree()?;

    let mut opts = git2::DiffOptions::new();

    let diff = context
        .git_repo
        .diff_tree_to_workdir(Some(&reference_tree), Some(&mut opts))?;

    Ok(diff)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AnalysisResult {
    Fresh,
    Dirty,
}

pub struct AnalysisCallbacks<T> {
    rust_file_modified: fn(&AnalysisContext, DiffDelta, T) -> AnalysisResult,
    rust_file_modified_hunk: fn(&AnalysisContext, DiffDelta, DiffHunk, T) -> AnalysisResult,

    other_file_modified: fn(&AnalysisContext, DiffDelta, T) -> AnalysisResult,
    other_file_modified_hunk: fn(&AnalysisContext, DiffDelta, DiffHunk, T) -> AnalysisResult,
}

impl<T> Clone for AnalysisCallbacks<T> {
    fn clone(&self) -> Self {
        Self {
            rust_file_modified: self.rust_file_modified,
            rust_file_modified_hunk: self.rust_file_modified_hunk,
            other_file_modified: self.other_file_modified,
            other_file_modified_hunk: self.other_file_modified_hunk,
        }
    }
}
impl<T> Copy for AnalysisCallbacks<T> {}

fn is_rust_file(p: &Path) -> bool {
    p.extension() == Some(OsStr::new("rs"))
}

pub fn delta_is_rust_file(delta: &DiffDelta) -> bool {
    delta.new_file().path().map_or(false, is_rust_file)
}

type DiffAnalysisFileCb<T> =
    fn(AnalysisCallbacks<T>, &AnalysisContext, DiffDelta) -> AnalysisResult;

pub struct DiffAnalysisCallbacks<T> {
    file_unmodified: DiffAnalysisFileCb<T>,
    file_added: DiffAnalysisFileCb<T>,
    file_deleted: DiffAnalysisFileCb<T>,
    file_modified: DiffAnalysisFileCb<T>,
    file_renamed: DiffAnalysisFileCb<T>,
    file_copied: DiffAnalysisFileCb<T>,
    file_ignored: DiffAnalysisFileCb<T>,
    file_untracked: DiffAnalysisFileCb<T>,
    file_typechange: DiffAnalysisFileCb<T>,
    file_unreadable: DiffAnalysisFileCb<T>,
    file_conflicted: DiffAnalysisFileCb<T>,
    hunk: fn(AnalysisCallbacks<T>, &AnalysisContext, DiffDelta, DiffHunk) -> AnalysisResult,
}

impl<T> Clone for DiffAnalysisCallbacks<T> {
    fn clone(&self) -> Self {
        Self {
            file_unmodified: self.file_unmodified,
            file_added: self.file_added,
            file_deleted: self.file_deleted,
            file_modified: self.file_modified,
            file_renamed: self.file_renamed,
            file_copied: self.file_copied,
            file_ignored: self.file_ignored,
            file_untracked: self.file_untracked,
            file_typechange: self.file_typechange,
            file_unreadable: self.file_unreadable,
            file_conflicted: self.file_conflicted,
            hunk: self.hunk,
        }
    }
}
impl<T> Copy for DiffAnalysisCallbacks<T> {}

fn for_delta_file<T>(
    delta: DiffDelta<'_>,
    diff_cb: DiffAnalysisCallbacks<T>,
    cb: AnalysisCallbacks<T>,
    analysis_context: &AnalysisContext,
    analysis_result: &Rc<RefCell<AnalysisResult>>,
) -> bool
where
    T: for<'a> From<&'a DiffDelta<'a>> + 'static,
{
    let diff_file_cb = match delta.status() {
        Delta::Unmodified => diff_cb.file_unmodified,
        Delta::Added => diff_cb.file_added,
        Delta::Deleted => diff_cb.file_deleted,
        Delta::Modified => diff_cb.file_modified,
        Delta::Renamed => diff_cb.file_renamed,
        Delta::Copied => diff_cb.file_copied,
        Delta::Ignored => diff_cb.file_ignored,
        Delta::Untracked => diff_cb.file_untracked,
        Delta::Typechange => diff_cb.file_typechange,
        Delta::Unreadable => diff_cb.file_unreadable,
        Delta::Conflicted => diff_cb.file_conflicted,
    };

    let r = diff_file_cb(cb, analysis_context, delta);
    if r == AnalysisResult::Dirty {
        *analysis_result.borrow_mut() = AnalysisResult::Dirty;
        return false;
    }

    true
}

fn for_delta_hunk<T>(
    delta: DiffDelta<'_>,
    hunk: DiffHunk<'_>,
    diff_cb: DiffAnalysisCallbacks<T>,
    cb: AnalysisCallbacks<T>,
    analysis_context: &AnalysisContext,
    analysis_result: &Rc<RefCell<AnalysisResult>>,
) -> bool
where
    T: for<'a> From<&'a DiffDelta<'a>> + 'static,
{
    let r = (diff_cb.hunk)(cb, analysis_context, delta, hunk);

    if r == AnalysisResult::Dirty {
        *analysis_result.borrow_mut() = AnalysisResult::Dirty;
        return false;
    }

    true
}

pub fn analysis_context_analyze<T>(
    analysis_context: &AnalysisContext,
    diff: Diff<'_>,
    cb: AnalysisCallbacks<T>,
    diff_cb: DiffAnalysisCallbacks<T>,
) -> DifftestsResult<AnalysisResult>
where
    T: for<'a> From<&'a DiffDelta<'a>> + 'static,
{
    let analysis_result = Rc::new(RefCell::new(AnalysisResult::Fresh));

    let diff_r = diff.foreach(
        &mut |delta, _progress| {
            for_delta_file(delta, diff_cb, cb, analysis_context, &analysis_result)
        },
        None,
        Some(&mut |delta, hunk| {
            for_delta_hunk(delta, hunk, diff_cb, cb, analysis_context, &analysis_result)
        }),
        None,
    );

    let r = *analysis_result.borrow();

    if let Err(e) = diff_r {
        return if e.code() == git2::ErrorCode::User {
            debug_assert_eq!(r, AnalysisResult::Dirty);
            Ok(r)
        } else {
            Err(e.into())
        };
    }

    Ok(r)
}

pub struct CbFileData {
    is_rust: bool,
}

impl<'a> From<&'a DiffDelta<'a>> for CbFileData {
    fn from(value: &'a DiffDelta<'a>) -> Self {
        Self {
            is_rust: delta_is_rust_file(value),
        }
    }
}

fn rust_file_modified(
    cx: &AnalysisContext,
    delta: DiffDelta<'_>,
    data: CbFileData,
) -> AnalysisResult {
    let _ = (cx, delta);
    if !data.is_rust {
        unreachable!()
    }

    AnalysisResult::Fresh
}

fn other_file_modified(
    cx: &AnalysisContext,
    delta: DiffDelta<'_>,
    data: CbFileData,
) -> AnalysisResult {
    let _ = (cx, delta);
    if data.is_rust {
        unreachable!()
    }

    AnalysisResult::Fresh
}

pub struct LineRange {
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.start <= other.start && self.end >= other.start
            || self.start <= other.end && self.end >= other.end
    }
}

fn rust_file_modified_hunk(
    cx: &AnalysisContext,
    delta: DiffDelta<'_>,
    hunk: DiffHunk<'_>,
    data: CbFileData,
) -> AnalysisResult {
    if !data.is_rust {
        unreachable!()
    }

    let path = delta.new_file().path().unwrap();

    for cm in &cx.data.data {
        for f in &cm.functions {
            let file = &f.filenames[0];

            if !file.ends_with(path) {
                continue;
            }

            for r in &f.regions {
                if r.execution_count != 0
                    && LineRange::new(r.l1, r.l2).intersects(&LineRange::new(
                        hunk.old_start() as usize,
                        (hunk.old_start() + hunk.old_lines() - 1) as usize,
                    ))
                {
                    return AnalysisResult::Dirty;
                }
            }
        }
    }

    AnalysisResult::Fresh
}

fn other_file_modified_hunk(
    cx: &AnalysisContext,
    delta: DiffDelta<'_>,
    hunk: DiffHunk<'_>,
    data: CbFileData,
) -> AnalysisResult {
    if data.is_rust {
        unreachable!()
    }

    let path = delta.new_file().path().unwrap();
    let path = path.to_str().unwrap();

    // let r = cx.other_file_modified_hunk(path, hunk);

    AnalysisResult::Fresh
}

pub fn standard_analysis_callbacks() -> AnalysisCallbacks<CbFileData> {
    AnalysisCallbacks {
        rust_file_modified,
        other_file_modified,
        rust_file_modified_hunk,
        other_file_modified_hunk,
    }
}

pub fn standard_diff_analysis_callbacks() -> DiffAnalysisCallbacks<CbFileData> {
    // FIXME: handle other cases
    // for now, given the code compiles, the other cases do not do anything.
    DiffAnalysisCallbacks {
        file_unmodified: |_, _, _| AnalysisResult::Fresh,
        file_added: |_, _, _| AnalysisResult::Fresh,
        file_deleted: |_, _, _| AnalysisResult::Fresh,
        file_modified: |cb, cx, delta| {
            let file_data = CbFileData::from(&delta);

            let cb = match file_data.is_rust {
                true => cb.rust_file_modified,
                false => cb.other_file_modified,
            };

            cb(cx, delta, file_data)
        },
        file_renamed: |_, _, _| AnalysisResult::Fresh,
        file_copied: |_, _, _| AnalysisResult::Fresh,
        file_ignored: |_, _, _| AnalysisResult::Fresh,
        file_untracked: |_, _, _| AnalysisResult::Fresh,
        file_typechange: |_, _, _| AnalysisResult::Fresh,
        file_unreadable: |_, _, _| AnalysisResult::Fresh,
        file_conflicted: |_, _, _| AnalysisResult::Fresh,
        hunk: |cb, cx, delta, hunk| {
            let file_data = CbFileData::from(&delta);

            let cb = match file_data.is_rust {
                true => cb.rust_file_modified_hunk,
                false => cb.other_file_modified_hunk,
            };

            cb(cx, delta, hunk, file_data)
        },
    }
}
