#![deny(clippy::all)]

use std::collections::{ HashMap, HashSet };
use swc_ecma_ast::*;
use swc_ecma_visit::{ VisitMut, VisitMutWith };
use swc_common::util::take::Take;
use swc_common::DUMMY_SP;

/*
    A struct to clean up any Walnut imports and do various other things for final pass
*/
pub(crate) struct WalnutFinalize {
    pub resolver_imports_to_remove: HashSet<String>,
    pub resolver_locs: HashMap<String, String>,
}

impl WalnutFinalize {
    pub fn new(resolver_imports_to_remove: HashSet<String>) -> Self {
        WalnutFinalize {
            resolver_imports_to_remove,
            resolver_locs: HashMap::new(),
        }
    }

    fn remove_import_id(&mut self, decl: &mut ImportDecl, id_string: &String) {
        if
            let Some(idx) = decl.specifiers.iter().position(|val| {
                match val {
                    ImportSpecifier::Named(v) => { id_string == &String::from(&*v.local.sym) }
                    ImportSpecifier::Default(v) => { id_string == &String::from(&*v.local.sym) }
                    ImportSpecifier::Namespace(v) => { id_string == &String::from(&*v.local.sym) }
                }
            })
        {
            decl.specifiers.swap_remove(idx);
        }
    }

    fn check_import_id(&mut self, decl: &mut ImportDecl) {
        let mut removed_something = false;
        for s in decl.specifiers.clone().iter() {
            match s {
                ImportSpecifier::Named(v) => {
                    let id_string = String::from(&*v.local.sym);
                    if self.resolver_imports_to_remove.contains(&id_string) {
                        self.remove_import_id(decl, &id_string);
                        removed_something = true;
                        self.resolver_locs.insert(id_string, String::from(&*decl.src.value));
                    }
                }
                ImportSpecifier::Default(v) => {
                    let id_string = String::from(&*v.local.sym);
                    if self.resolver_imports_to_remove.contains(&id_string) {
                        self.remove_import_id(decl, &id_string);
                        removed_something = true;
                        self.resolver_locs.insert(id_string, String::from(&*decl.src.value));
                    }
                }
                ImportSpecifier::Namespace(v) => {
                    let id_string = String::from(&*v.local.sym);
                    if self.resolver_imports_to_remove.contains(&id_string) {
                        self.remove_import_id(decl, &id_string);
                        removed_something = true;
                        self.resolver_locs.insert(id_string, String::from(&*decl.src.value));
                    }
                }
            }
        }

        if !removed_something {
            return;
        }
        if decl.specifiers.is_empty() {
            decl.take();
        }
    }
}

impl VisitMut for WalnutFinalize {
    fn visit_mut_import_decl(&mut self, n: &mut ImportDecl) {
        self.check_import_id(n);
    }

    fn visit_mut_var_declarators(&mut self, n: &mut Vec<VarDeclarator>) {
        n.visit_mut_children_with(self);

        n.retain(|node| {
            if node.name.is_invalid() {
                return false;
            }
            true
        })
    }

    fn visit_mut_stmt(&mut self, n: &mut Stmt) {
        n.visit_mut_children_with(self);

        match n {
            Stmt::Decl(Decl::Var(var)) => {
                if var.decls.is_empty() {
                    *n = Stmt::Empty(EmptyStmt { span: DUMMY_SP });
                }
            }
            _ => {}
        }
    }

    fn visit_mut_stmts(&mut self, n: &mut Vec<Stmt>) {
        n.visit_mut_children_with(self);

        n.retain(|s| !matches!(s, Stmt::Empty(..)));
    }

    fn visit_mut_module_items(&mut self, n: &mut Vec<ModuleItem>) {
        n.visit_mut_children_with(self);

        n.retain(|s| {
            match s {
                ModuleItem::ModuleDecl(ModuleDecl::Import(x)) => !x.src.is_empty(),
                ModuleItem::Stmt(Stmt::Empty(..)) => false,
                _ => true,
            }
        });
    }
}
