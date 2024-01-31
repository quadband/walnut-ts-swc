#![deny(clippy::all)]

use swc_ecma_ast::*;
use swc_ecma_visit::VisitMut;
use swc_common::util::take::Take;

use crate::transform::WalnutSymbols;

pub(crate) struct ScanFirst {
    pub should_run: bool,
    pub run_val: bool,
    pub run_resolve: bool,
    pub run_jsx: bool,
}

impl ScanFirst {
    pub fn new() -> Self {
        ScanFirst {
            should_run: false,
            run_val: false,
            run_resolve: false,
            run_jsx: false,
        }
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
            for spec in n.specifiers.iter() {
                if let Some(s) = spec.as_named() {
                    match &*s.local.sym {
                        WalnutSymbols::VAL | WalnutSymbols::PVAL => {
                            self.run_val = true;
                        }
                        WalnutSymbols::RESOLVE => {
                            self.run_resolve = true;
                        }
                        "$Walnut" => {
                            self.run_jsx = true;
                        }
                        _ => {}
                    }
                }
            }
            n.take();
            self.should_run = true;
        }
    }
}
