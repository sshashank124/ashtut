use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

#[proc_macro_derive(GlslStruct)]
pub fn derive_macro_glsl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let name_str = name.to_string();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(ref fields),
            ..
        }) => fields.named.iter().map(|field| {
            let field_name_str = field.ident.as_ref().unwrap().to_string();
            let field_type = &field.ty;

            quote! {
                ::glsl::GlslField {
                    name: #field_name_str,
                    ty: <#field_type as ::glsl::Glsl>::NAME,
                }
            }
        }),
        _ => unimplemented!(),
    };

    quote! {
        impl #impl_generics ::glsl::Glsl for #name #ty_generics #where_clause {
            const NAME: &'static str = #name_str;
        }

        impl #impl_generics ::glsl::GlslStruct for #name #ty_generics #where_clause {
            const FIELDS: &'static [::glsl::GlslField] = &[
                #( #fields, )*
            ];
        }
    }
    .into()
}
