use proc_macro::TokenStream;
use syn::{parse::Parse, parse_macro_input, FnArg, ItemFn, Type};

struct FilterArg {
    guard: Type,
}

impl Parse for FilterArg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        if ident != "guard" {
            return Err(syn::Error::new_spanned(
                ident,
                "expected `guard = <Type>` in #[filter(...)]",
            ));
        }
        input.parse::<syn::Token![=]>()?;
        let guard: Type = input.parse()?;
        Ok(FilterArg { guard })
    }
}

pub fn filter_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as FilterArg);
    apply_filter(args.guard, item)
}

/// Shared implementation used by `#[filter(...)]` and the convenience filter macros.
pub fn apply_filter(guard_ty: Type, item: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(item as ItemFn);

    // Prepend `_: GuardType` to the function signature.
    let guard_param: FnArg = syn::parse_quote! {
        _: #guard_ty
    };
    item_fn.sig.inputs.insert(0, guard_param);

    let output = quote::quote! {
        #[allow(unused_variables)]
        #item_fn
    };

    output.into()
}
