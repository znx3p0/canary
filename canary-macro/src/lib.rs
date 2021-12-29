use proc_macro::{Span, TokenStream};
use quote::quote;
use syn::{spanned::Spanned, ImplItem, ItemFn, ItemImpl, ItemStruct, LitStr, Type};

macro_rules! panic_span {
    ($span: expr, $msg: expr) => {
        return syn::Error::new($span.span(), $msg)
            .to_compile_error()
            .into()
    };
}

/// services run on a global cluster that can be exposed through providers.
/// services are an agnostic way of communicating.
///
/// At first sight, they might look similar to HTTP handlers, and although they are similar,
/// it is important to note that there are various differences.
///
/// A service represents a pipeline of objects through which objects may be sent or received.
///
/// HTTP handlers are stuck in TCP, services can use TCP, Unix and whatever providers are available.
/// HTTP handlers are based on the request-response architecture, services are stream-based.
///
/// Current providers are: TCP, TCP(unencrypted / raw), Unix(unencrypted / raw).
/// Future support for UDP is planned.
///
/// ```rust
/// #[service] // if no pipeline is indicated, the type of the channel must be specified
/// async fn my_service(chan: Channel) -> Result<()> {
///     tx!(chan, 8); // send number
///     rx!(number, chan); // receive number
///     println!("received number {}", number);
///     Ok(())
/// }
/// ```
///
/// services can also have metadata
/// ```rust
/// #[service] // if no pipeline is indicated, the type of the channel must be specified
/// async fn my_counter(counter: Arc<AtomicU64>, mut chan: Channel) -> Result<()> {
///     let val = counter.fetch_add(1, Ordering::Relaxed);
///     chan.tx(val).await?;
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn service(attrs: TokenStream, tokens: TokenStream) -> TokenStream {
    let mut item = syn::parse_macro_input!(tokens as ItemFn);
    let vis = &item.vis;

    let mut has_pipeline = true;
    let pipeline = match syn::parse::<Type>(attrs) {
        Ok(t) => t,
        Err(_) => {
            has_pipeline = false;
            Type::Verbatim(quote!(()))
        }
    };

    let endpoint = LitStr::new(
        &format!("{}", item.sig.ident.clone()),
        Span::call_site().into(),
    );

    let name = item.sig.ident.clone();
    if let None = item.sig.asyncness {
        panic_span!(item.sig, "function has to be async");
    }

    let mut meta = quote!(());

    if item.sig.inputs.len() == 1 {
        item.sig
            .inputs
            .insert(0, syn::parse2(quote!(_: ())).unwrap())
    }

    // infers type whenever a pipeline is defined
    // and allows for metadata
    match has_pipeline {
        true => {
            if let syn::FnArg::Typed(s) = item.sig.inputs.first_mut().unwrap() {
                let ty = *s.ty.clone();
                if quote!(_).to_string() == quote!(#ty).to_string() {
                    //  type inferred
                    s.ty =
                        Box::new(syn::parse2(quote!(::canary::comms::Typed<#pipeline>)).unwrap());
                }
            }
        }
        false => {
            let mut index = 0;
            item.sig.inputs.iter_mut().for_each(|s| {
                match index {
                    0 => {
                        if let syn::FnArg::Typed(s) = s {
                            let ty = &s.ty;
                            meta = quote!(#ty);
                        }
                    }
                    1 => {
                        if let syn::FnArg::Typed(s) = s {
                            let ty = *s.ty.clone();
                            if quote!(_).to_string() == quote!(#ty).to_string() {
                                //  type inferred
                                s.ty = Box::new(
                                    syn::parse2(quote!(::canary::comms::Typed<#pipeline>)).unwrap(),
                                );
                            }
                        }
                    }
                    _ => panic!("functions cannot have more than two parameters"),
                }
                if let syn::FnArg::Typed(s) = s {
                    let ty = *s.ty.clone();
                    if quote!(_).to_string() == quote!(#ty).to_string() {
                        //  type inferred
                        s.ty = Box::new(syn::parse2(quote!(::canary::Typed<#pipeline>)).unwrap());
                    }
                }
                index += 1;
            });
        }
    }

    quote!(
        #[allow(non_camel_case_types)]
        #vis struct #name;
        impl ::canary::service::Service for #name {
            const ENDPOINT: &'static str = #endpoint;
            type Pipeline = #pipeline;
            type Meta = #meta;
            fn service(__canary_inner_meta: #meta) -> Box<dyn Fn(::canary::igcp::BareChannel) + Send + Sync + 'static> {
                #item
                ::canary::service::run_metadata(__canary_inner_meta, #name)
            }
        }
    )
    .into()
}

#[proc_macro_attribute]
pub fn route(attrs: TokenStream, tokens: TokenStream) -> TokenStream {
    // syn::Error::new(, "asdf")
    //         .to_compile_error();
    match syn::parse::<ItemStruct>(tokens.clone()) {
        Ok(s) => struct_route(attrs, s),
        Err(_) => {
            let p = syn::parse::<ItemImpl>(tokens).unwrap();
            impl_route(attrs, p)
        }
    }
}

#[proc_macro_attribute]
pub fn main(_: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::ItemFn);

    let ret = &input.sig.output;
    let name = &input.sig.ident;
    let inputs = &input.sig.inputs;

    if input.sig.asyncness.is_none() {
        let msg = "the async keyword is missing from the function declaration";
        return syn::Error::new_spanned(input.sig.fn_token, msg)
            .to_compile_error()
            .into();
    } else if name == "main" && !inputs.is_empty() {
        let msg = "the main function cannot accept arguments";
        return syn::Error::new_spanned(&input.sig.inputs, msg)
            .to_compile_error()
            .into();
    }

    quote!(
        fn main() #ret {
            #input
            ::canary::runtime::block_on(main())
        }
    )
    .into()
}

fn struct_route(attrs: TokenStream, item: ItemStruct) -> TokenStream {
    let ident = &item.ident;
    let endpoint = syn::parse::<LitStr>(attrs)
        .unwrap_or(LitStr::new(&ident.clone().to_string(), ident.span()));
    quote!(
        #item
        impl ::canary::routes::RegisterEndpoint for #ident {
            const ENDPOINT: &'static str = #endpoint;
        }
    )
    .into()
}

fn impl_route(_attrs: TokenStream, mut item: ItemImpl) -> TokenStream {
    let methods = item
        .items
        .clone()
        .into_iter()
        .map(|s| match s {
            ImplItem::Method(method) => Some(method),
            _ => None,
        })
        .filter(|s| s.is_some())
        .map(|s| s.unwrap());
    item.attrs.clear();

    let names = methods.map(|method| method.sig.ident).collect::<Vec<_>>();
    let name = &item.self_ty;

    quote!(
        const _: () = {
            #item

            #(
                #[::canary::service]
                async fn #names(__inner: ::std::sync::Arc<#name>, channel: ::canary::Channel) -> ::canary::Result<()> {
                    __inner.#names(channel).await
                }
            )*


            impl ::canary::routes::Register for #name {
                type Meta = ::std::sync::Arc<Self>;
                fn register(top_route: &::canary::routes::Route, meta: Self::Meta) -> ::canary::Result<()> {
                    #(
                        top_route.add_service::<#names>(meta.clone())?;
                    )*
                    Ok(())
                }
            }
        };

    ).into()
}
