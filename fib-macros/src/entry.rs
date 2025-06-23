use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};


pub(crate) fn main_impl(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let ItemFn {
        attrs,
        vis,
        sig,
        block,
    } = input;

    let stmts = block.stmts;
    let ident = sig.ident.clone();


    quote! {
        #(#attrs)*
        #vis #sig {
            let __rt = fib::runtime::runtime();
            let __result = __rt.block_on(|| {
                #(#stmts)*
            });
            __result
        }
    }.into()
}