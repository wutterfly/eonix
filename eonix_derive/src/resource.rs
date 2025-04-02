use proc_macro::TokenStream;
use syn::DeriveInput;

pub fn impl_trait_resource(ast: DeriveInput) -> TokenStream {
    let ident = ast.ident;

    quote::quote! {
        impl Resource for #ident {}
    }
    .into()
}
