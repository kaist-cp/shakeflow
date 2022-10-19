use proc_macro::{self, TokenStream};
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, GenericParam, Lifetime, LifetimeDef};

use super::utils::get_member_symbol;

pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let vis = &ast.vis;
    let mut generics_lifetime = ast.generics.clone();
    generics_lifetime.params.push(GenericParam::Lifetime(LifetimeDef::new(Lifetime::new("'id", Span::call_site()))));
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();
    let (impl_generics_lifetime, ty_generics_lifetime, where_clause_lifetime) = generics_lifetime.split_for_impl();
    let name = &ast.ident;
    let pname = format!("{}Proj", name);
    let pident = syn::Ident::new(&pname, name.span());
    let fields = match ast.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }), ..
        }) => named,
        syn::Data::Enum(_) => {
            // NOTE: return empty `TokenStream` for enum, since we will not use projection
            return TokenStream::default();
        }
        _ => todo!("expr projection should panic for Union Types"),
    };

    let proj_struct_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;
        quote! { #vis #name: Expr<'id, #ty> }
    });

    // TODO: Remove redundant clone
    let proj_fields = fields.iter().enumerate().map(|(idx, f)| {
        let name = &f.ident;
        quote! { #name: Expr::member(value.clone(), #idx) }
    });

    let unproj_fields = fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap();
        let symbol = get_member_symbol(&f.attrs, name);

        match symbol {
            None => quote! { (None, projected.#name.into_inner()) },
            Some(symbol) => {
                quote! { (Some(#symbol.to_string()), projected.#name.into_inner()) }
            }
        }
    });

    let fields_count = fields.iter().count();

    let setter_fields = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_ty = &f.ty;

        let setter_ident = syn::Ident::new(&format!("set_{}", field_name), field_name.span());

        if fields_count == 1 {
            quote! {
                #[allow(missing_docs)]
                #vis fn #setter_ident(self, #field_name: Expr<'id, #field_ty>) -> Expr<'id, #name #ty_generics> {
                    #pident {
                        #field_name,
                    }.into()
                }
            }
        } else {
            quote! {
                #[allow(missing_docs)]
                #vis fn #setter_ident(self, #field_name: Expr<'id, #field_ty>) -> Expr<'id, #name #ty_generics> {
                    #pident {
                        #field_name,
                        ..self
                    }.into()
                }
            }
        }
    });

    let expanded = quote! {
        #[allow(missing_docs)]
        #[derive(Debug, Clone)]
        #vis struct #pident #impl_generics_lifetime #where_clause {
            #(#proj_struct_fields,)*
        }
        impl #impl_generics ExprProj for #name #ty_generics #where_clause {
            type Target<'id> = #pident #ty_generics_lifetime;
            fn proj<'id>(value: Expr<'id, Self>) -> Self::Target<'id> {
                Self::Target::<'id> {
                    #(#proj_fields,)*
                }
            }
        }
        impl #impl_generics_lifetime From<#pident #ty_generics_lifetime> for Expr<'id, #name #ty_generics> #where_clause {
            fn from(projected: #pident #ty_generics_lifetime) -> Expr<'id, #name #ty_generics> {
                Expr::from(lir::Expr::Struct {
                    inner: vec![
                        #(#unproj_fields,)*
                    ]
                })
            }
        }
        impl #impl_generics_lifetime Copy for #pident #ty_generics_lifetime #where_clause_lifetime {}
        impl #impl_generics_lifetime #pident #ty_generics_lifetime #where_clause_lifetime {
            #(#setter_fields)*
        }
        impl #impl_generics_lifetime lir::TableStorageElement<'id> for #pident #ty_generics_lifetime #where_clause_lifetime {}
    };
    expanded.into()
}
