use proc_macro::{self, TokenStream};
use proc_macro2::Span;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::{Colon, Const};
use syn::{
    parse_macro_input, Attribute, ConstParam, DeriveInput, Ident, Lit, LitInt, Path, PathArguments, PathSegment, Type,
    TypePath,
};

pub(super) fn get_enum_width(attrs: &[Attribute]) -> Option<LitInt> {
    for attr in attrs {
        if let Ok(syn::Meta::List(nvs)) = attr.parse_meta() {
            if nvs.path.get_ident().unwrap() == "width" {
                return nvs.nested.iter().find_map(|nv| match nv {
                    syn::NestedMeta::Lit(syn::Lit::Int(width)) => Some(width.clone()),
                    _ => None,
                });
            }
        }
    }
    None
}

pub(super) fn get_enum_encode_value(attrs: &[Attribute]) -> Option<LitInt> {
    for attr in attrs {
        if let Ok(syn::Meta::List(nvs)) = attr.parse_meta() {
            if nvs.path.get_ident().unwrap() == "encode" {
                return nvs.nested.iter().find_map(|nv| match nv {
                    syn::NestedMeta::Lit(syn::Lit::Int(width)) => Some(width.clone()),
                    _ => None,
                });
            }
        }
    }
    None
}

pub(super) fn get_member_symbol(attrs: &[Attribute], name: &Ident) -> Option<Lit> {
    for attr in attrs {
        let meta = match attr.parse_meta() {
            Ok(syn::Meta::List(nvs)) => {
                assert_eq!(nvs.path.get_ident().unwrap(), "member");

                nvs.nested.iter().find_map(|nv| match nv {
                    syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) => {
                        if nv.path.get_ident().unwrap() == "name" {
                            Some(nv.lit.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
            }
            _ => continue,
        };

        if meta.is_none() {
            return Some(Lit::new(proc_macro2::Literal::string(&format!("{}", name))));
        }
        let meta = meta.unwrap();

        return match meta {
            Lit::Str(ref s) => {
                if s.value().is_empty() {
                    None
                } else {
                    Some(meta)
                }
            }
            lit => panic!("expected string, found {:?}", lit),
        };
    }
    Some(Lit::new(proc_macro2::Literal::string(&format!("{}", name))))
}

pub(super) fn get_member_sep(attrs: &[Attribute]) -> Option<Lit> {
    for attr in attrs {
        let meta = match attr.parse_meta() {
            Ok(syn::Meta::List(nvs)) => {
                assert_eq!(nvs.path.get_ident().unwrap(), "member");

                nvs.nested.iter().find_map(|nv| match nv {
                    syn::NestedMeta::Meta(syn::Meta::NameValue(nv)) => {
                        if nv.path.get_ident().unwrap() == "sep" {
                            Some(nv.lit.clone())
                        } else {
                            None
                        }
                    }
                    syn::NestedMeta::Meta(syn::Meta::Path(path)) => {
                        if path.get_ident().unwrap() == "nosep" {
                            Some(Lit::new(proc_macro2::Literal::string("")))
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
            }
            _ => continue,
        };

        meta.as_ref()?;
        let meta = meta.unwrap();

        return match meta {
            Lit::Str(ref s) => {
                if s.value().is_empty() {
                    Some(Lit::new(proc_macro2::Literal::string("")))
                } else {
                    Some(meta)
                }
            }
            lit => panic!("expected string, found {:?}", lit),
        };
    }
    None
}

pub(super) fn get_var_arr_struct(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let vis = &ast.vis;
    let generics = {
        let mut segments = Punctuated::new();
        segments
            .push_value(PathSegment { ident: Ident::new("usize", Span::call_site()), arguments: PathArguments::None });

        let const_param = ConstParam {
            attrs: vec![],
            const_token: Const::default(),
            ident: Ident::new("ELTS", Span::call_site()),
            colon_token: Colon::default(),
            ty: Type::Path(TypePath { qself: None, path: Path { leading_colon: None, segments } }),
            eq_token: None,
            default: None,
        };

        let mut generics = ast.generics.clone();
        generics.params.push(const_param.into());
        generics
    };
    let (impl_generics_arr, _, where_clause) = generics.split_for_impl();
    let name = &ast.ident;
    let pname = format!("{}VarArr", name);
    let pident = syn::Ident::new(&pname, name.span());
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
        return TokenStream::default();
    };

    let var_arr_struct_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        if is_bool_type(ty) {
            quote! { #vis #name: Array<bool, num::U<ELTS>> }
        } else {
            quote! { #vis #name: VarArray<#ty, num::U<ELTS>> }
        }
    });

    let expanded = quote! {
        #[allow(missing_docs)]
        #[derive(Debug, Clone)]
        #vis struct #pident #impl_generics_arr #where_clause {
            #(#var_arr_struct_fields,)*
        }
    };
    expanded.into()
}

pub(super) fn get_var_arr_impls(input: TokenStream, input_var_arr: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let vis = &ast.vis;
    let name = &ast.ident;
    let pname = format!("{}Proj", name);
    let pident = syn::Ident::new(&pname, name.span());
    let (_, ty_generics, _) = ast.generics.split_for_impl();
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
        todo!()
    };

    let ast_var_arr = parse_macro_input!(input_var_arr as DeriveInput);
    let name_var_arr = &ast_var_arr.ident;
    let pname_var_arr = format!("{}Proj", name_var_arr);
    let pident_var_arr = syn::Ident::new(&pname_var_arr, name_var_arr.span());
    let (impl_generics_var_arr, ty_generics_var_arr, where_clause) = ast_var_arr.generics.split_for_impl();

    let new_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        if is_bool_type(ty) {
            quote! { #name: 0.into() }
        } else {
            quote! { #name: Expr::x() }
        }
    });

    let get_entry_fields = fields.iter().map(|f| {
        let name = &f.ident;

        quote! { #name: value.#name[index] }
    });

    let expanded = quote! {
        #[allow(missing_docs)]
        impl #impl_generics_var_arr #name_var_arr #ty_generics_var_arr #where_clause {
            #vis fn new_expr<'id>() -> Expr<'id, Self> {
                #pident_var_arr {
                    #(#new_fields,)*
                }
                .into()
            }
            #vis fn get_entry<'id>(value: Expr<'id, Self>, index: Expr<'id, Bits<num::Log2<num::U<ELTS>>>>) -> Expr<'id, #name #ty_generics> {
                #pident {
                    #(#get_entry_fields,)*
                }
                .into()
            }
        }
    };
    expanded.into()
}

pub(super) fn is_bool_type(ty: &Type) -> bool {
    if let Type::Path(ty_path) = ty {
        let segments = &ty_path.path.segments;
        segments.len() == 1 && segments[0].ident == "bool"
    } else {
        false
    }
}
