use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

use super::utils::{get_enum_encode_value, get_enum_width, get_member_symbol};

pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let name = &ast.ident;
    match ast.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }), ..
        }) => {
            let fields = named;

            let ty_widths = fields.iter().map(|f| {
                let ty = &f.ty;
                quote! { <#ty>::WIDTH }
            });

            // fields for `transl`.
            let into_fields = fields.iter().map(|f| {
                let name = &f.ident;
                quote! { .chain(self.#name.transl().iter()) }
            });

            // fields for `port_decls`.
            let port_decls_fields = fields.iter().map(|f| {
                let name = f.ident.as_ref().unwrap();
                let ty = &f.ty;
                let symbol = get_member_symbol(&f.attrs, name);

                match symbol {
                    None => quote! { (None, <#ty>::port_decls()) },
                    Some(symbol) => quote! { (Some(#symbol.to_string()), <#ty>::port_decls()) },
                }
            });

            let expanded = quote! {
                impl #impl_generics Signal for #name #ty_generics #where_clause {
                    const WIDTH: usize = #(#ty_widths)+*;
                    fn transl(self) -> Vec<bool> {
                        ::std::iter::empty()#(#into_fields)*.copied().collect::<Vec<bool>>()
                    }
                    fn port_decls() -> lir::PortDecls {
                        lir::PortDecls::Struct(vec![
                            #(#port_decls_fields,)*
                        ])
                    }
                }
            };

            expanded.into()
        }
        syn::Data::Enum(syn::DataEnum { ref variants, .. }) => {
            fn clog2(value: usize) -> usize {
                if value == 0 {
                    0
                } else {
                    (::std::mem::size_of::<usize>() * 8) - (value - 1).leading_zeros() as usize
                }
            }
            let variant_count = variants.iter().count();
            assert!(variant_count > 0, "{name}: Empty enums cannot be derived as shakeflow `Signal`");
            let width = if let Some(width) = get_enum_width(&ast.attrs) {
                width.base10_parse::<usize>().unwrap_or_else(|_| panic!("{name}: Enum width should be usize"))
            } else if variant_count == 1 {
                1
            } else {
                clog2(variant_count)
            };

            let typ_width = quote! {#width};

            let into_variants = variants.iter().enumerate().map(|(i, f)| {
                let variant_name = &f.ident;
                assert!(
                    matches!(f.fields, syn::Fields::Unit),
                    "{name}::{variant_name}: Only Unit Variant is allowed to be derived as Shakeflow Signal"
                );

                let encode_value = if let Some(encode_value_lit) = get_enum_encode_value(&f.attrs) {
                    let encode_value = encode_value_lit
                        .base10_parse::<usize>()
                        .unwrap_or_else(|_| panic!("encoding value of {name}::{variant_name} should be usize"));
                    assert!(
                        encode_value < (1 << width),
                        "{encode_value}(encoding of {name}::{variant_name}) exceeds maximum for {width} bits",
                    );
                    encode_value
                } else {
                    i
                };

                quote! { Self::#variant_name => (0..#width).map(|idx| {
                    ((#encode_value >> idx) & 1) != 0
                }).collect::<Vec<bool>>(), }
            });

            let expanded = quote! {
                impl #impl_generics Signal for #name #ty_generics #where_clause {
                    const WIDTH: usize = #typ_width;
                    fn transl(self) -> Vec<bool> {
                        match self {
                            #(#into_variants)*
                        }
                    }
                    fn port_decls() -> lir::PortDecls {
                        lir::PortDecls::Bits(lir::Shape::new([Self::WIDTH]))
                    }
                }

                impl EnumValue for #name {}
            };

            expanded.into()
        }
        _ => todo!("Signal macro is not implemented for union type"),
    }
}
