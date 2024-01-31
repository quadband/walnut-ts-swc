#![deny(clippy::all)]

use core::panic;
use std::{ collections::{ HashMap, HashSet }, path::{ Path, PathBuf }, str };
use swc::{ resolver, PrintArgs };
use swc_atoms::Atom;
use swc_common::{
    collections::AHashMap,
    errors::{ ColorConfig, Handler },
    sync::Lrc,
    util::take::Take,
    FileName,
    SourceMap,
    DUMMY_SP,
};
use swc_ecma_ast::*;
use swc_ecma_loader::{ resolve::Resolve, TargetEnv };
use swc_ecma_parser::{ Syntax, TsConfig };
use swc_ecma_visit::{ Visit, VisitMut, VisitMutWith, VisitWith };

use crate::scan_first::ScanFirst;

pub(crate) struct WalnutSymbols;
impl WalnutSymbols {
    pub const VAL: &'static str = "$Val";
    pub const PVAL: &'static str = "$PVal";
    pub const RESOLVE: &'static str = "$Resolve";
}

/*
    The main Walnut Transform struct.
*/
struct WalnutTransform {
    walnut_key: String,
    resolver_ids: Vec<String>,
    is_in_jsx: bool,
}

impl WalnutTransform {
    pub fn new(walnut_key: String) -> Self {
        WalnutTransform {
            walnut_key: walnut_key,
            resolver_ids: Vec::new(),
            is_in_jsx: false,
        }
    }

    fn transform_tool(&mut self, e: &mut CallExpr) -> Option<Expr> {
        match e.callee.clone() {
            Callee::Expr(callee) =>
                match *callee {
                    Expr::Ident(i) =>
                        match &*i.sym {
                            WalnutSymbols::VAL | WalnutSymbols::PVAL => self.transform_val(e),
                            WalnutSymbols::RESOLVE => self.setup_resolve(e),
                            _ => None,
                        }
                    _ => None,
                }
            _ => None,
        }
    }

    fn transform_val(&mut self, e: &mut CallExpr) -> Option<Expr> {
        let val_obj = {
            let mut v = ObjectLitFinder::new();
            e.visit_with(&mut v);
            match v.res {
                Some(v) => v,
                None => {
                    return None;
                }
            }
        };

        let matched_val = self.extract_val(&val_obj);

        matched_val
    }

    fn extract_val(&mut self, val_obj: &ObjectLit) -> Option<Expr> {
        let mut matched_val: Option<Expr> = None;

        for prop in val_obj.props.iter() {
            let p = match prop {
                PropOrSpread::Prop(prop) => *prop.clone(),
                _ => { panic!("Spread operators disallowed.") }
            };

            let (prop_name, val) = match p.key_value() {
                Some(v) => (v.key, *v.value.clone()),
                _ => panic!("No value found for prop"),
            };

            let key = match prop_name {
                PropName::Ident(k) => String::from(&*k.sym),
                PropName::Str(k) => k.value.to_string(),
                PropName::Num(k) => k.to_string(),
                PropName::BigInt(k) => k.value.to_string(),
                _ => panic!("Keys must be of type string or number"),
            };

            if key == self.walnut_key {
                matched_val = Some(val);
                break;
            }
        }

        if matched_val == None {
            let first_prop = match val_obj.props.get(0) {
                Some(v) => v,
                None => {
                    return None;
                }
            };

            let p = match first_prop {
                PropOrSpread::Prop(v) => *v.clone(),
                _ => panic!("Spread operators disallowed."),
            };

            let val = match p.key_value() {
                Some(v) => *v.value.clone(),
                _ => panic!("No value found for prop"),
            };

            matched_val = Some(val);
        }

        matched_val
    }

    fn setup_resolve(&mut self, e: &mut CallExpr) -> Option<Expr> {
        let arg = match e.args.get(0) {
            Some(v) => v.clone(),
            None => {
                return None;
            }
        };

        let resolver_id = match *arg.expr {
            Expr::Ident(i) => {
                //(i.to_id(), String::from(&*i.sym))
                String::from(&*i.sym)
            }
            _ => {
                return None;
            }
        };

        let mark_string = format!("/* __wres_{resolver_id} */");

        let mark_val = Atom::new(mark_string);

        let marker = Expr::Lit(
            Lit::Str(Str {
                span: DUMMY_SP,
                value: mark_val.clone(),
                raw: Some(mark_val.clone()),
            })
        );

        self.resolver_ids.push(resolver_id);

        Some(marker)
    }

    fn is_valid_jsx_identifier(&mut self, name: &JSXElementName) -> bool {
        match name {
            JSXElementName::Ident(s) =>
                match &*s.sym {
                    "$Walnut" => true,
                    _ => false,
                }
            _ => false,
        }
    }

    fn handle_element(&mut self, element: &JSXElement) -> bool {
        //if !self.is_valid_jsx_identifier(&element.opening.name){ return }

        let Some(key_vec) = element.opening.attrs.iter().find_map(|jsx_attr_or_spread| {
            match jsx_attr_or_spread {
                JSXAttrOrSpread::JSXAttr(
                    JSXAttr { name: JSXAttrName::Ident(Ident { sym, .. }), value, .. },
                ) if sym == "key" => {
                    match value {
                        Some(val) => {
                            match val {
                                JSXAttrValue::JSXExprContainer(jsx_expr_cont) => {
                                    match jsx_expr_cont.expr.clone() {
                                        JSXExpr::Expr(e) => {
                                            match *e.clone() {
                                                Expr::Array(arr) => {
                                                    let mut ret_vec = Vec::<String>::new();
                                                    for itm in arr.elems {
                                                        match itm {
                                                            Some(expr_or_sprd) => {
                                                                match *expr_or_sprd.expr.clone() {
                                                                    Expr::Lit(Lit::Str(s)) =>
                                                                        ret_vec.push(
                                                                            String::from(&*s.value)
                                                                        ),
                                                                    _ => {
                                                                        return None;
                                                                    }
                                                                }
                                                            }
                                                            None => {
                                                                return None;
                                                            }
                                                        }
                                                    }
                                                    Some(ret_vec)
                                                }
                                                Expr::Lit(Lit::Str(s)) => {
                                                    Some(
                                                        Vec::<String>::from([
                                                            String::from(&*s.value),
                                                        ])
                                                    )
                                                }
                                                _ => None,
                                            }
                                        }
                                        _ => None,
                                    }
                                }
                                JSXAttrValue::Lit(Lit::Str(s)) => {
                                    Some(Vec::<String>::from([String::from(&*s.value)]))
                                }
                                _ => None,
                            }
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }) else {
            return false;
        };

        //println!("{:?}", key_val_expr);

        let mut matches_walnut_key: bool = false;

        for key in key_vec {
            if key == self.walnut_key {
                matches_walnut_key = true;
            }
        }

        //println!("{}", matches_walnut_key);

        matches_walnut_key
    }

    fn handle_fragment(&mut self, frag: &mut JSXFragment) {
        frag.visit_mut_children_with(self);
    }
}

impl VisitMut for WalnutTransform {
    fn visit_mut_var_declarator(&mut self, n: &mut VarDeclarator) {
        let mut walnut_call_expr = {
            let mut v = WalnutFinder::new();
            n.visit_with(&mut v);
            match v.res {
                Some(v) => v,
                None => {
                    return;
                }
            }
        };

        let new_node = self.transform_tool(&mut walnut_call_expr);

        // In case we get back None for whatever reason (we shouldn't, we should panic) we mark as invalid.
        match new_node {
            Some(expr) => {
                n.init = Some(Box::new(expr));
            }
            None => {
                n.name = Pat::Invalid(Invalid { span: DUMMY_SP });
            }
        }
    }

    fn visit_mut_jsx_fragment(&mut self, n: &mut JSXFragment) {
        if self.is_in_jsx {
            return;
        }
        self.is_in_jsx = true;
        n.visit_mut_children_with(self);
        self.is_in_jsx = false;
    }

    fn visit_mut_jsx_element(&mut self, n: &mut JSXElement) {
        let old_is_in_jsx = self.is_in_jsx;
        self.is_in_jsx = true;

        n.visit_mut_children_with(self);
        if n.closing == None {
            self.is_in_jsx = old_is_in_jsx;
            return;
        }

        let mut new_children: Vec<JSXElementChild> = Vec::new();

        for child in n.children.iter() {
            match child {
                JSXElementChild::JSXFragment(frag) => {
                    self.handle_fragment(&mut frag.clone());
                    new_children.push(child.clone());
                }
                JSXElementChild::JSXElement(el) => {
                    if !self.is_valid_jsx_identifier(&el.opening.name) {
                        new_children.push(child.clone());
                        continue;
                    }
                    if self.handle_element(&el) {
                        for el_ch in el.children.iter() {
                            new_children.push(el_ch.clone());
                        }
                    }
                }
                _ => {
                    new_children.push(child.clone());
                }
            }
        }

        n.children = new_children;
        self.is_in_jsx = old_is_in_jsx;
    }
}

/*
    A helper struct to find a usage of a Walnut Function undeneath a variable declaration.
*/
struct WalnutFinder {
    res: Option<CallExpr>,
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
struct ObjectLitFinder {
    res: Option<ObjectLit>,
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

/*
    A struct to clean up any Walnut imports and do various other things for final pass
*/
struct WalnutFinalize {
    resolver_imports_to_remove: HashSet<String>,
    resolver_locs: HashMap<String, String>,
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

/*
    Handles the creation of the compiler and running stuff.
    Will also return the source code.
*/

#[napi]
pub struct WalnutHandler {
    compiler: swc::Compiler,
    program: Program,
    walnut_key: String,
    input_code: String,
    output_code: Option<String>,
    pub need_resolver: bool,
    resolver_labels: Vec<String>,
    label_map: HashMap<String, String>,
    entry_id: String,
}

#[napi]
impl WalnutHandler {
    pub fn new(code: String, id: String, walnut_key: String) -> Self {
        let cm = Lrc::<SourceMap>::default();
        let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

        let compiler = swc::Compiler::new(cm.clone());

        let fm = cm.new_source_file(FileName::Custom(id.clone()), code.clone());

        let program = compiler
            .parse_js(
                fm.clone(),
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

        WalnutHandler {
            compiler,
            program,
            walnut_key,
            input_code: code,
            output_code: None,
            need_resolver: false,
            entry_id: id,
            resolver_labels: Vec::new(),
            label_map: HashMap::new(),
        }
    }

    #[napi]
    pub fn run(&mut self) {
        let mut scan_first = ScanFirst::new();
        self.program.visit_mut_with(&mut scan_first);

        if !scan_first.should_run {
            self.output_code = Some(self.input_code.clone());
            return;
        }

        // Transform pass
        let mut w_trans = WalnutTransform::new(self.walnut_key.clone());
        self.program.visit_mut_with(&mut w_trans);

        if w_trans.resolver_ids.len() > 0 {
            self.need_resolver = true;
        }

        // Final pass for cleanup and stuff
        let mut resolver_hash_set: HashSet<String> = HashSet::new();
        // Loop over the returned vec to generate a hash map
        for id in w_trans.resolver_ids.iter() {
            resolver_hash_set.insert(id.clone());
        }

        let mut w_finalize = WalnutFinalize::new(resolver_hash_set);
        self.program.visit_mut_with(&mut w_finalize);

        if w_finalize.resolver_locs.len() > 0 {
            let resolved_labels = try_resolve_resolver_label(
                w_finalize.resolver_locs,
                &self.entry_id
            );

            // We do this to call any resolver that dynamically returns a result.
            for id in w_trans.resolver_ids.iter() {
                let Some(v) = resolved_labels.get(id) else {
                    return;
                };
                self.resolver_labels.push(v.clone());
                self.label_map.insert(v.clone(), id.to_owned());
            }
        }
    }

    #[napi]
    pub fn get_output(&mut self) -> String {
        if self.output_code == None {
            let printed_code = self.compiler.print(&self.program, PrintArgs {
                source_root: None,
                source_file_name: None,
                output_path: None,
                inline_sources_content: false,
                source_map: Default::default(),
                orig: None,
                comments: None,
                emit_source_map_columns: false,
                preamble: "",
                codegen_config: Default::default(),
                ..Default::default()
            });

            let output = match printed_code {
                Ok(v) => v,
                _ => panic!("idk at this point"),
            };

            self.output_code = Some(output.code);
        }

        self.output_code.clone().unwrap()
    }

    #[napi]
    pub fn get_resolver_labels(&mut self) -> Vec<String> {
        self.resolver_labels.clone()
    }

    #[napi]
    pub fn satisfy_resolvers(&mut self, resolver_arr: Vec<(String, String)>) {
        let mut code = self.get_output();

        for reso in resolver_arr {
            let (label, value) = reso;
            let fn_name = match self.label_map.get(&label) {
                Some(v) => v.clone(),
                None => {
                    return;
                }
            };

            let mark = format!("/* __wres_{fn_name} */");
            code = code.replacen(&*mark, &*value, 1);
        }

        self.output_code = Some(code);
    }
}

fn try_resolve_resolver_label(
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
