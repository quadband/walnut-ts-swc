#![deny(clippy::all)]

use std::{ collections::HashMap, path::{ Path, PathBuf } };
use swc::resolver;
use swc_common::{
    collections::AHashMap,
    errors::{ ColorConfig, Handler },
    sync::Lrc,
    FileName,
    SourceMap,
};
use swc_ecma_ast::*;
use swc_ecma_loader::{
    resolve::Resolve,
    TargetEnv,
    resolvers::{ tsc::TsConfigResolver, lru::CachingResolver, node::NodeModulesResolver },
};
use swc_ecma_parser::{ Syntax, TsConfig };
use swc_ecma_visit::{ Visit, VisitWith };

// fn get_file_resolver(
//   base: &String
// ) -> CachingResolver<TsConfigResolver<NodeModulesResolver>> {
//   resolver::paths_resolver(
//       TargetEnv::Node,
//       AHashMap::default(),
//       PathBuf::from(base.clone()),
//       Vec::new(),
//       true
//   )
// }

fn get_file_resolver(base: &String) -> CachingResolver<TsConfigResolver<NodeModulesResolver>> {
  let r = TsConfigResolver::new(
    NodeModulesResolver::new(
      TargetEnv::Node,
      AHashMap::default(),
      true
    ),
    PathBuf::from(base.clone()),
    Vec::new(),
);

  let cr: CachingResolver<TsConfigResolver<NodeModulesResolver>> = CachingResolver::new(
    40,
    r
  );

  cr
}

pub(crate) fn resolve_deps(base: &String, entry_id: &String) {
    let cm = Lrc::<SourceMap>::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));
    let compiler = swc::Compiler::new(cm.clone());

    let fm = cm.load_file(Path::new(&entry_id.to_string())).expect("Something gone wrong.");

    let program = compiler
        .parse_js(
            fm,
            &handler,
            EsVersion::Es2020,
            Syntax::Typescript(TsConfig {
                tsx: true,
                decorators: false,
                dts: false,
                no_early_errors: true,
                disallow_ambiguous_jsx_like: true,
            }),
            swc::config::IsModule::Bool(true),
            Default::default()
        )
        .expect("Failed to parse");

    let mut import_finder = ImportFinder::new(base, entry_id);

    program.visit_with(&mut import_finder);
}

struct ImportFinder {
    file_resolver: CachingResolver<TsConfigResolver<NodeModulesResolver>>,
    entry: String,
    base: String
}

impl ImportFinder {
    pub fn new(base: &String, entry: &String) -> Self {
        ImportFinder {
            file_resolver: get_file_resolver(base),
            entry: entry.clone(),
            base: base.clone()
        }
    }
}

impl Visit for ImportFinder {
    fn visit_import_decl(&mut self, n: &ImportDecl) {
        let base = FileName::Real(PathBuf::from(&self.entry));
        let resolved = self.file_resolver.resolve(&base, &*n.src.value);
        println!("{:?}", resolved);
    }
}
