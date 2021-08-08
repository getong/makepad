use makepad_live_parser::LiveError;
use makepad_live_parser::Span;
//use makepad_live_parser::LiveValue;
use makepad_live_parser::LiveErrorOrigin;
use makepad_live_parser::live_error_origin;
//use crate::shaderast::IdentPath;
use crate::shaderast::Ty;
use crate::shaderast::Param;
use crate::shaderast::Ident;
use crate::shaderast::Block;
use crate::shaderast::Expr;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::cell::RefCell;

type Scope = HashMap<Ident, Sym>;

#[derive(Clone, Debug)]
pub enum ClosureDef{
    Block{
        span:Span,
        params: Vec<Ident>,
        block: Block,
    },
    Expr{
        span:Span,
        params: Vec<Ident>,
        expr: Expr
    },
}

#[derive(Clone, Debug)]
pub struct Env {
    //pub live_uniform_deps: RefCell<Option<BTreeSet<(Ty, FullNodePtr) >> >,
    pub scopes: Vec<Scope>,
    pub closures: RefCell<Vec<ClosureDef>>
}


impl Env {
    pub fn new() -> Env {
        Env {
            closures: RefCell::new(Vec::new()),
            scopes: Vec::new(),
        }
    }
    
    pub fn find_sym_on_scopes(&self, ident: Ident, _span: Span,) -> Option<Sym> {
        
        let ret = self.scopes.iter().rev().find_map( | scope | scope.get(&ident));
        if ret.is_some() {
            return Some(ret.unwrap().clone())
        }
        return None
    }
    
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new())
    }
    
    pub fn pop_scope(&mut self) {
        self.scopes.pop().unwrap();
    }
    
    pub fn insert_sym(&mut self, span: Span, ident: Ident, sym: Sym) -> Result<(), LiveError> {
        match self.scopes.last_mut().unwrap().entry(ident) {
            Entry::Vacant(entry) => {
                entry.insert(sym);
                Ok(())
            }
            Entry::Occupied(_) => Err(LiveError {
                origin:live_error_origin!(),
                span,
                message: format!("`{}` is already defined in this scope", ident),
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Sym {
    Local{
        is_mut: bool, 
        ty: Ty, 
    },
    Closure{
        return_ty: Ty,
        params: Vec<Param>
    }
}