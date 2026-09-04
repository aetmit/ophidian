use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Error, Fields, LitStr, Result, Type, parse_macro_input, parse_quote};

pub fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let generics = &input.generics;

    let rhs_type: Type = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("add_assign"))
        .map(parse_rhs_attribute)
        .transpose()
        .unwrap()
        .unwrap_or_else(|| parse_quote!(Self));

    let body = match &input.data {
        Data::Struct(data) => {
            match &data.fields {
                Fields::Unit => {
                    quote! {
                        // adding a unit struct with a value do
                    }
                }
                Fields::Unnamed(fields) => {
                    if fields.unnamed.len() != 1 {
                        return {
                            quote! {
                                compile_error!("AddAssign requires exactly 1 field");
                            }.into()
                        }
                    }

                    quote! {
                        self.0 += rhs
                    }
                }
                Fields::Named(fields) => {
                    if fields.named.len() != 1 {
                        return {
                            quote! {
                                compile_error!("AddAssign requires exactly 1 field");
                            }.into()
                        }
                    }

                    let field = fields.named.first().unwrap();
                    let field_name = field.ident.as_ref().unwrap();

                    quote! {
                        self.#field_name += rhs;
                    }
                }
            }
        }
        _ => {
            return quote! {
                compile_error!("AddAssign can only be derived for structs");
            }.into();
        }
    };

    let (impl_generics, ty_generics, where_clause) =
        generics.split_for_impl();

    quote! {
        impl #impl_generics std::ops::AddAssign<#rhs_type>
            for #name #ty_generics #where_clause
        {
            fn add_assign(&mut self, rhs: #rhs_type) {
                #body
            }
        }
    }
    .into()
}

fn parse_rhs_attribute(attr: &Attribute) -> Result<Type> {
    let mut rhs = None;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("rhs") {
            let value: LitStr = meta.value()?.parse()?;
            rhs = Some(value.parse()?);
            Ok(())
        } else {
            Err(meta.error("expected `rhs = \"...\""))
        }
    })?;

    rhs.ok_or_else(|| Error::new_spanned(attr, "missing 'rhs'"))
}
