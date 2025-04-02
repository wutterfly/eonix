mod component;
mod nosend;
mod resource;

use proc_macro::TokenStream;
use syn::DeriveInput;

#[proc_macro_derive(Resource)]
pub fn derive_resource(tokens: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(tokens).unwrap();

    resource::impl_trait_resource(ast)
}

#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(tokens: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(tokens).unwrap();

    component::impl_trait_component(ast)
}

#[proc_macro_derive(NoSend)]
pub fn derive_system_nosend(tokens: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(tokens).unwrap();

    nosend::impl_trait_nosend(ast)
}
