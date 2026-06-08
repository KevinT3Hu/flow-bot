use proc_macro::TokenStream;
mod filter;
mod service;

#[proc_macro_attribute]
pub fn flow_service(_attr: TokenStream, item: TokenStream) -> TokenStream {
    service::flow_service_macro(item)
}

#[proc_macro_attribute]
pub fn flow_filter(attr: TokenStream, item: TokenStream) -> TokenStream {
    filter::filter_macro(attr, item)
}

/// Equivalent to `#[filter(guard = ::flow_bot::extract::filters::IsGroupMessage)]`.
#[proc_macro_attribute]
pub fn group_message(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let guard = syn::parse_quote!(::flow_bot::extract::filters::IsGroupMessage);
    filter::apply_filter(guard, item)
}

/// Equivalent to `#[filter(guard = ::flow_bot::extract::filters::IsPrivateMessage)]`.
#[proc_macro_attribute]
pub fn private_message(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let guard = syn::parse_quote!(::flow_bot::extract::filters::IsPrivateMessage);
    filter::apply_filter(guard, item)
}
