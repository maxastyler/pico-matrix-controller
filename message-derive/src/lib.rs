use quote::quote;
use syn::{parse_macro_input, DeriveInput};



#[proc_macro_derive(WebsocketMessage)]
pub fn message_derive_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput {
        attrs,
        vis,
        ident,
        generics,
        data,
    } = parse_macro_input!(input as DeriveInput);

    let expanded = quote! {
    impl #ident {
        fn oops() {}
    }
    };
    expanded.into()
}
