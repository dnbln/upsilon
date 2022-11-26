use crate::ast::Ident;
use std::cell::RefCell;
use std::rc::Rc;

use crate::defs::Defs;
use crate::diagnostics::{DiagnosticsHost, Label};
use crate::lower::{
    LowerBuiltinTy, LowerEnum, LowerEnumVariant, LowerFile, LowerNewtypeStruct, LowerPackage,
    LowerPackageItem, LowerPath, LowerStruct, LowerStructField, LowerTyRef, RefTargetHost,
};
use crate::span::{Span, SpanHosts, TextSize};

struct CompileContext {
    span_hosts: Rc<SpanHosts>,
    references: Rc<RefCell<crate::lower::References>>,
    file: Rc<LowerFile>,

    def_collection_successful: RefCell<bool>,
    ref_resolve_successful: RefCell<bool>,
    current_package_scope: RefCell<Rc<LowerPath>>,
}

fn tombstone_path(span_hosts: Rc<SpanHosts>) -> LowerPath {
    LowerPath::Ident(Ident::new(
        "tombstone".to_string(),
        Span::new(TextSize::ZERO, TextSize::ZERO, span_hosts),
    ))
}

pub(crate) fn compile(file: LowerFile, diagnostics: &DiagnosticsHost) -> Option<Defs> {
    let cx = CompileContext {
        span_hosts: Rc::clone(&file.span_hosts),
        references: Rc::clone(&file.references),
        current_package_scope: RefCell::new(Rc::new(tombstone_path(Rc::clone(&file.span_hosts)))),
        file: Rc::new(file),

        def_collection_successful: RefCell::new(true),
        ref_resolve_successful: RefCell::new(true),
    };

    collect_defs_for_file(&cx, &cx.file);

    if !*cx.def_collection_successful.borrow() {
        return None;
    }

    resolve_references_for_file(&cx, &cx.file);

    if !*cx.ref_resolve_successful.borrow() {
        return None;
    }

    let mut packages = vec![];

    Some(Defs { packages })
}

fn collect_defs_for_file(cx: &CompileContext, file: &LowerFile) {
    for package in file.packages.iter() {
        collect_defs_for_package(cx, package, None);
    }
}

fn collect_defs_for_package(
    cx: &CompileContext,
    package: &Rc<LowerPackage>,
    parent: Option<Rc<LowerPath>>,
) {
    let self_path = Rc::new(match parent {
        Some(parent) => LowerPath::Path(
            Rc::clone(&parent),
            package.package_kw.span().clone().into(),
            package.name.clone(),
        ),
        None => LowerPath::Ident(package.name.clone()),
    });

    {
        duplicate_definition_check_or_register(
            cx,
            self_path.as_ref(),
            RefTargetHost::Package(Rc::clone(package)),
        );
    }

    for item in package.package_items.iter() {
        match item {
            LowerPackageItem::NewtypeStruct(newtype_struct) => {
                collect_def_for_newtype_struct(cx, newtype_struct, Rc::clone(&self_path))
            }
            LowerPackageItem::Struct(struct_) => {
                collect_def_for_struct(cx, struct_, Rc::clone(&self_path))
            }
            LowerPackageItem::Enum(enum_) => collect_def_for_enum(cx, enum_, Rc::clone(&self_path)),
            LowerPackageItem::Package(package) => {
                collect_defs_for_package(cx, package, Some(Rc::clone(&self_path)))
            }
        }
    }
}

fn collect_def_for_newtype_struct(
    cx: &CompileContext,
    newtype_struct: &Rc<LowerNewtypeStruct>,
    parent: Rc<LowerPath>,
) {
    let self_path = LowerPath::Path(
        parent,
        newtype_struct.struct_kw.span().clone().into(),
        newtype_struct.name.clone(),
    );

    duplicate_definition_check_or_register(
        cx,
        self_path,
        RefTargetHost::NewtypeStruct(Rc::clone(newtype_struct)),
    );
}

fn collect_def_for_struct(cx: &CompileContext, struct_: &Rc<LowerStruct>, parent: Rc<LowerPath>) {
    let self_path = LowerPath::Path(
        parent,
        struct_.struct_kw.span().clone().into(),
        struct_.name.clone(),
    );

    duplicate_definition_check_or_register(
        cx,
        self_path,
        RefTargetHost::Struct(Rc::clone(struct_)),
    );
}

fn collect_def_for_enum(cx: &CompileContext, enum_: &Rc<LowerEnum>, parent: Rc<LowerPath>) {
    let self_path = LowerPath::Path(
        parent,
        enum_.enum_kw.span().clone().into(),
        enum_.name.clone(),
    );

    duplicate_definition_check_or_register(cx, self_path, RefTargetHost::Enum(Rc::clone(enum_)));
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Successful {
    Yes,
    No,
}

pub trait LowerPathHolder {
    fn get(&self) -> &LowerPath;

    fn into_owned(self) -> LowerPath;
}

impl LowerPathHolder for LowerPath {
    fn get(&self) -> &LowerPath {
        self
    }

    fn into_owned(self) -> LowerPath {
        self
    }
}

impl<'a> LowerPathHolder for &'a LowerPath {
    fn get(&self) -> &LowerPath {
        *self
    }

    fn into_owned(self) -> LowerPath {
        self.clone()
    }
}

impl LowerPathHolder for Rc<LowerPath> {
    fn get(&self) -> &LowerPath {
        self.as_ref()
    }

    fn into_owned(self) -> LowerPath {
        self.get().clone()
    }
}

fn duplicate_definition_check_or_register<T>(
    cx: &CompileContext,
    self_path: T,
    target: RefTargetHost,
) -> Successful
where
    T: LowerPathHolder,
{
    let mut refs = cx.references.borrow_mut();

    let self_path_ref = self_path.get();

    if let Some(old_target) = refs.get(self_path_ref) {
        target
            .name_span()
            .error("duplicate definition")
            .with_message(format!("Duplicate definition of {self_path_ref}"))
            .with_additional_label(Label::new(
                old_target.name_span().clone(),
                "previous definition here",
            ))
            .emit();

        *cx.def_collection_successful.borrow_mut() = false;

        return Successful::No;
    }

    refs.push_new_ref(self_path.into_owned(), target);

    Successful::Yes
}

fn resolve_references_for_file(cx: &CompileContext, file: &LowerFile) {
    for package in file.packages.iter() {
        resolve_references_for_package(cx, package, None);
    }
}

fn resolve_references_for_package(
    cx: &CompileContext,
    package: &LowerPackage,
    parent: Option<Rc<LowerPath>>,
) {
    let self_path = Rc::new(match parent {
        Some(parent) => LowerPath::Path(
            Rc::clone(&parent),
            package.package_kw.span().clone().into(),
            package.name.clone(),
        ),
        None => LowerPath::Ident(package.name.clone()),
    });

    let old_path = std::mem::replace(
        &mut *cx.current_package_scope.borrow_mut(),
        Rc::clone(&self_path),
    );

    struct RestoreOldPath<'cx> {
        old_path: Rc<LowerPath>,
        cx: &'cx CompileContext,
    }

    impl<'cx> Drop for RestoreOldPath<'cx> {
        fn drop(&mut self) {
            if !::std::thread::panicking() {
                std::mem::swap(
                    &mut *self.cx.current_package_scope.borrow_mut(),
                    &mut self.old_path,
                );

                // self.old_path now has the self_path above.
                // RefCell::borrow_mut() may panic
            }
        }
    }

    let _guard = RestoreOldPath { old_path, cx };

    for item in package.package_items.iter() {
        match item {
            LowerPackageItem::NewtypeStruct(newtype_struct) => {
                resolve_references_for_newtype_struct(cx, newtype_struct, Rc::clone(&self_path))
            }
            LowerPackageItem::Struct(struct_) => {
                resolve_references_for_struct(cx, struct_, Rc::clone(&self_path))
            }
            LowerPackageItem::Enum(enum_) => {
                resolve_references_for_enum(cx, enum_, Rc::clone(&self_path))
            }
            LowerPackageItem::Package(package) => {
                resolve_references_for_package(cx, package, Some(Rc::clone(&self_path)))
            }
        }
    }
}

fn resolve_references_for_newtype_struct(
    cx: &CompileContext,
    newtype_struct: &LowerNewtypeStruct,
    parent: Rc<LowerPath>,
) {
    let self_path = Rc::new(LowerPath::Path(
        parent,
        newtype_struct.newtype_kw.span().clone().into(),
        newtype_struct.name.clone(),
    ));

    resolve_references_for_type(cx, &newtype_struct.raw_ty, &self_path);
}

fn resolve_references_for_struct(
    cx: &CompileContext,
    struct_: &LowerStruct,
    parent: Rc<LowerPath>,
) {
    let self_path = Rc::new(LowerPath::Path(
        parent,
        struct_.struct_kw.span().clone().into(),
        struct_.name.clone(),
    ));

    for field in struct_.fields.iter() {
        resolve_references_for_field(cx, field, Rc::clone(&self_path));
    }
}

fn resolve_references_for_enum(cx: &CompileContext, enum_: &LowerEnum, parent: Rc<LowerPath>) {
    let self_path = Rc::new(LowerPath::Path(
        parent,
        enum_.enum_kw.span().clone().into(),
        enum_.name.clone(),
    ));

    for variant in enum_.variants.iter() {
        resolve_references_for_enum_variant(cx, variant, Rc::clone(&self_path));
    }
}

fn resolve_references_for_field(
    cx: &CompileContext,
    field: &LowerStructField,
    parent: Rc<LowerPath>,
) {
    resolve_references_for_type(cx, &field.ty, &parent);
}

fn resolve_references_for_enum_variant(
    cx: &CompileContext,
    variant: &LowerEnumVariant,
    parent: Rc<LowerPath>,
) {
    resolve_references_for_type(cx, &variant.ty, &parent);
}

fn resolve_references_for_type(cx: &CompileContext, ty: &LowerTyRef, parent_path: &Rc<LowerPath>) {
    let target = &ty.path;

    match resolve_path(cx, target, parent_path) {
        None => {
            ty.path
                .span()
                .error("here")
                .with_message(format!("unknown type: {}", ty.path))
                .emit();

            *cx.ref_resolve_successful.borrow_mut() = false;
        }
        Some(target) => {
            ty.path_ref.resolved_to(target.clone());
        }
    }

    if let Some(generics) = &ty.generics {
        for generic in &generics.ty_params {
            resolve_references_for_type(cx, generic, parent_path);
        }
    }
}

fn resolve_path(
    cx: &CompileContext,
    path: &Rc<LowerPath>,
    parent_path: &Rc<LowerPath>,
) -> Option<RefTargetHost> {
    fn find_package<'a>(cx: &CompileContext, path: &'a Rc<LowerPath>) -> Option<&'a Rc<LowerPath>> {
        match cx.references.borrow().get(path) {
            Some(
                RefTargetHost::Struct(_) | RefTargetHost::Enum(_) | RefTargetHost::NewtypeStruct(_),
            ) => find_package(cx, path.unwrap_parent()),
            Some(RefTargetHost::Package(_)) => Some(path),
            Some(RefTargetHost::BuiltinType(_)) | None => None,
        }
    }

    if let LowerPath::Ident(ident) = &**path {
        if let Some(builtin_ty) = LowerBuiltinTy::for_name(ident.as_str()) {
            return Some(RefTargetHost::BuiltinType(builtin_ty));
        }
    }

    let package = find_package(cx, parent_path)?;

    if let Some(target) = cx.references.borrow_mut().get(&path) {
        return Some(target.clone());
    }

    if let Some(target) = cx.references.borrow_mut().get(&package.join(path)) {
        return Some(target.clone());
    }

    None
}
