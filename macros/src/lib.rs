mod addassign;
mod ast;
mod constructor;

use proc_macro::TokenStream;

#[proc_macro_derive(Constructor)]
pub fn constructor(input: TokenStream) -> TokenStream {
    constructor::expand(input)
}

#[proc_macro_derive(AddAssign, attributes(add_assign))]
pub fn add_assign(input: TokenStream) -> TokenStream {
    addassign::expand(input)
}

#[proc_macro_derive(Ast)]
pub fn ast(input: TokenStream) -> TokenStream {
    ast::expand(input)
}
