#![deny(clippy::all)]

use swc_ecma_ast::*;
use swc_ecma_visit::VisitMut;
use swc_common::util::take::Take;

pub(crate) struct ScanFirst {
    pub should_run: bool
}

impl ScanFirst {
    pub fn new() -> Self {
        ScanFirst { should_run: false }
    }

    fn check_if_walnut_import(&mut self, src: &str) -> bool {
        if src.contains("walnut-ts") {
            return true;
        }
        false
    }
}

impl VisitMut for ScanFirst {
    fn visit_mut_import_decl(&mut self, n: &mut ImportDecl) {
        if self.check_if_walnut_import(&*n.src.value) {
            n.take();
            self.should_run = true;
        }
    }
}

