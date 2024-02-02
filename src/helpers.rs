#![deny(clippy::all)]

use swc_ecma_ast::*;
use swc_ecma_visit::Visit;

use crate::transform::WalnutSymbols;

/*
    A helper struct to find a usage of a Walnut Function undeneath a variable declaration.
*/
pub(crate) struct WalnutFinder {
  pub res: Option<CallExpr>,
}

impl WalnutFinder {
  pub fn new() -> Self {
      WalnutFinder { res: None }
  }

  fn is_valid_identifier(e: &Expr) -> bool {
      match e {
          Expr::Ident(i) =>
              match &*i.sym {
                  WalnutSymbols::VAL | WalnutSymbols::PVAL | WalnutSymbols::RESOLVE => true,
                  _ => false,
              }
          _ => false,
      }
  }
}

impl Visit for WalnutFinder {
  fn visit_call_expr(&mut self, n: &CallExpr) {
      match &n.callee {
          Callee::Expr(callee) if Self::is_valid_identifier(callee) => {}
          _ => {
              return;
          }
      }

      self.res = Some(n.clone());
  }
}

/*
  A little helper struct to grab the object literal in Val and PVal
*/
pub(crate) struct ObjectLitFinder {
  pub res: Option<ObjectLit>,
}

impl ObjectLitFinder {
  pub fn new() -> Self {
      ObjectLitFinder { res: None }
  }
}

impl Visit for ObjectLitFinder {
  fn visit_object_lit(&mut self, n: &ObjectLit) {
      self.res = Some(n.clone());
  }
}