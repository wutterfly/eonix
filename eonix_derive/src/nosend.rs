use proc_macro::TokenStream;
use syn::DeriveInput;

pub fn impl_trait_nosend(ast: DeriveInput) -> TokenStream {
    let ident = ast.ident;

    quote::quote! {
        impl NoSend for #ident {}
    }
    .into()
}
