use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

use spec::diagnostics::DiagnosticsHost;
use spec::lower::*;
use spec::Compiler;

pub struct SerdeCompiler;

impl Compiler for SerdeCompiler {
    fn compile_file(
        &self,
        cx: spec::CompileCx,
        file: &Rc<LowerFile>,
        diagnostics: &DiagnosticsHost,
        to: &Path,
    ) {
        compile_file(&CompileCx::new_from(&cx), file, diagnostics, to)
    }
}

struct CompileCx<'a> {
    spec_cx: &'a spec::CompileCx<'a>,

    definitions: Defs,
}

impl<'a> CompileCx<'a> {
    fn new_from(spec_cx: &'a spec::CompileCx<'a>) -> Self {
        Self {
            spec_cx,
            definitions: Defs {
                definitions: RefCell::new(Default::default()),
            },
        }
    }
}

struct Defs {
    definitions: RefCell<BTreeMap<LowerPath, Definition>>,
}

enum Definition {
    Enum(DefEnum),
    Struct(DefStruct),
    NewtypeStruct(DefNewtypeStruct),
}

struct DefEnum {}
struct DefStruct {}
struct DefNewtypeStruct {}

fn compile_file(cx: &CompileCx, file: &Rc<LowerFile>, diagnostics: &DiagnosticsHost, to: &Path) {
    for package in file.packages.iter() {
        compile_package(cx, package, diagnostics);
    }
}

fn compile_package(cx: &CompileCx, package: &Rc<LowerPackage>, diagnostics: &DiagnosticsHost) {
    for package_item in package.package_items.iter() {
        match package_item {
            LowerPackageItem::NewtypeStruct(_) => {}
            LowerPackageItem::Struct(_) => {}
            LowerPackageItem::Enum(_) => {}
            LowerPackageItem::Package(package) => {
                compile_package(cx, package, diagnostics);
            }
        }
    }
}
