use proc_macro::TokenStream;

use syn::DeriveInput;

pub fn impl_trait_component(ast: DeriveInput) -> TokenStream {
    let ident = ast.ident;

    quote::quote! {
        impl Component for #ident { }
    }
    .into()
}
