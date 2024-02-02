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
use swc_ecma_loader::{ resolve::Resolve, TargetEnv };
use swc_ecma_parser::{ Syntax, TsConfig };
use swc_ecma_visit::{ Visit, VisitWith };

pub(crate) fn try_resolve_resolver_label(
    resolver_locs: HashMap<String, String>,
    entry_id: &String
) -> HashMap<String, String> {
    let alias_map: AHashMap<String, String> = AHashMap::default();
    let base_url = PathBuf::from(entry_id.clone());
    let paths = Vec::new();

    let file_resolver = resolver::paths_resolver(TargetEnv::Node, alias_map, base_url, paths, true);

    let mut label_map: HashMap<String, String> = HashMap::new();

    for (id, path) in resolver_locs {
        let module_specifier = &*path;
        let base = FileName::Real(PathBuf::from(entry_id));
        let resolved = file_resolver.resolve(&base, module_specifier);

        let res_path = match resolved {
            Ok(res) => {
                //println!("{:?}", res.filename);
                res.filename
            }
            Err(e) => {
                println!("{:?}", e);
                continue;
            }
        };

        let search_id = id;

        let label = get_resolver_label(&search_id, res_path);

        match label {
            Some(s) => {
                label_map.insert(search_id, s);
            }
            None => {}
        }
    }

    label_map
}

fn get_resolver_label(search_id: &String, res_path: FileName) -> Option<String> {
    let cm = Lrc::<SourceMap>::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));
    let compiler = swc::Compiler::new(cm.clone());

    let fm = cm.load_file(Path::new(&res_path.to_string())).expect("Something gone wrong.");

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

    let mut label_finder = ResLabelFinder::new(search_id.clone());

    program.visit_with(&mut label_finder);

    label_finder.label
}

struct ResLabelFinder {
    search_id: String,
    label: Option<String>,
}

impl ResLabelFinder {
    pub fn new(search_id: String) -> Self {
        ResLabelFinder {
            search_id,
            label: None,
        }
    }
}

impl Visit for ResLabelFinder {
    fn visit_var_declarators(&mut self, n: &[VarDeclarator]) {
        for dec in n {
            let name = match dec.name.clone() {
                Pat::Ident(s) => String::from(&*s.sym),
                _ => {
                    return;
                }
            };

            if name == self.search_id {
                let mut v = LabelExtractor::new();
                n.visit_children_with(&mut v);

                self.label = v.res;
            }
        }
    }
}

struct LabelExtractor {
    res: Option<String>,
}

impl LabelExtractor {
    pub fn new() -> Self {
        LabelExtractor {
            res: None,
        }
    }

    fn is_valid_identifier(e: &Expr) -> bool {
        match e {
            Expr::Member(MemberExpr { obj, prop: MemberProp::Ident(prop), .. }) =>
                match &**obj {
                    Expr::Ident(i) => &*i.sym == "Walnut" && &*prop.sym == "makeResolver",
                    _ => false,
                }
            _ => false,
        }
    }
}

impl Visit for LabelExtractor {
    fn visit_call_expr(&mut self, n: &CallExpr) {
        match &n.callee {
            Callee::Expr(callee) if Self::is_valid_identifier(callee) => {}
            _ => {
                return;
            }
        }

        let label = match n.args.get(0) {
            Some(v) =>
                match *v.expr.clone() {
                    Expr::Lit(lit) =>
                        match lit {
                            Lit::Str(s) => String::from(&*s.value),
                            _ => {
                                return;
                            }
                        }
                    _ => {
                        return;
                    }
                }
            None => {
                return;
            }
        };

        self.res = Some(label);
    }
}
