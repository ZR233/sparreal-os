extern crate proc_macro;
#[macro_use]
extern crate quote;
extern crate core;
extern crate proc_macro2;
#[macro_use]
extern crate syn;

mod api_trait;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use syn::{
    parse, spanned::Spanned, FnArg, ImplItem, ItemFn, ItemImpl, ItemTrait, Pat, PathArguments, TraitItem, Type, Visibility
};

/// Attribute to declare the entry point of the program
///
/// **IMPORTANT**: This attribute must appear exactly *once* in the dependency graph. Also, if you
/// are using Rust 1.30 the attribute must be used on a reachable item (i.e. there must be no
/// private modules between the item and the root of the crate); if the item is in the root of the
/// crate you'll be fine. This reachability restriction doesn't apply to Rust 1.31 and newer releases.
///
/// The specified function will be called by the reset handler *after* RAM has been initialized.
/// If present, the FPU will also be enabled before the function is called.
///
/// The type of the specified function must be `[unsafe] fn() -> !` (never ending function)
///
/// # Properties
///
/// The entry point will be called by the reset handler. The program can't reference to the entry
/// point, much less invoke it.
///
/// # Examples
///
/// - Simple entry point
///
/// ``` no_run
/// # #![no_main]
/// # use sparreal_macros::entry;
/// #[entry]
/// fn main() -> ! {
///     loop {
///         /* .. */
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn entry(args: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as ItemFn);

    // check the function arguments
    if f.sig.inputs.len() > 3 {
        return parse::Error::new(
            f.sig.inputs.last().unwrap().span(),
            "`#[entry]` function has too many arguments",
        )
        .to_compile_error()
        .into();
    }
    for arg in &f.sig.inputs {
        match arg {
            FnArg::Receiver(_) => {
                return parse::Error::new(arg.span(), "invalid argument")
                    .to_compile_error()
                    .into();
            }
            FnArg::Typed(t) => {
                if !is_simple_type(&t.ty, "usize") {
                    return parse::Error::new(t.ty.span(), "argument type must be usize")
                        .to_compile_error()
                        .into();
                }
            }
        }
    }

    // check the function signature
    let valid_signature = f.sig.constness.is_none()
        && f.sig.asyncness.is_none()
        && f.vis == Visibility::Inherited
        && f.sig.abi.is_none()
        && f.sig.generics.params.is_empty()
        && f.sig.generics.where_clause.is_none()
        && f.sig.variadic.is_none()
        // && match f.sig.output {
        //     ReturnType::Default => false,
        //     ReturnType::Type(_, ref ty) => matches!(**ty, Type::Never(_)),
        // }
        ;

    if !valid_signature {
        return parse::Error::new(
            f.span(),
            "`#[entry]` function must have signature `[unsafe] fn([arg0: usize, ...]) `",
        )
        .to_compile_error()
        .into();
    }

    if !args.is_empty() {
        return parse::Error::new(Span::call_site(), "This attribute accepts no arguments")
            .to_compile_error()
            .into();
    }

    // XXX should we blacklist other attributes?
    let attrs = f.attrs;
    let unsafety = f.sig.unsafety;
    let args = f.sig.inputs;
    let stmts = f.block.stmts;

    quote!(
        #[allow(non_snake_case)]
        #[unsafe(no_mangle)]
        #(#attrs)*
        pub #unsafety extern "C" fn __sparreal_rt_main(#args) {
            #(#stmts)*
        }
    )
    .into()
}

#[allow(unused)]
fn is_simple_type(ty: &Type, name: &str) -> bool {
    if let Type::Path(p) = ty {
        if p.qself.is_none() && p.path.leading_colon.is_none() && p.path.segments.len() == 1 {
            let segment = p.path.segments.first().unwrap();
            if segment.ident == name && segment.arguments == PathArguments::None {
                return true;
            }
        }
    }
    false
}

fn func_ident(trait_ident: &Ident, func_ident: &Ident) -> Ident {
    let trait_ident = trait_ident.to_string().to_ascii_lowercase();
    format_ident!("__sparreal_api_{}_{}", trait_ident, func_ident)
}

#[proc_macro_attribute]
pub fn api_trait(_args: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as ItemTrait);

    let mut funcs = Vec::new();

    for item in &f.items {
        if let TraitItem::Fn(func) = item {
            let ident = func.sig.ident.clone();
            let inputs = func.sig.inputs.clone();
            let output = func.sig.output.clone();

            let api_name = func_ident(&f.ident, &ident);

            let mut args = Vec::new();

            for arg in &inputs {
                if let FnArg::Typed(t) = arg {
                    if let Pat::Ident(i) = t.pat.as_ref() {
                        let ident = &i.ident;
                        args.push(quote! { #ident , });
                    }
                }
            }
            funcs.push(quote! {

                pub unsafe fn #ident (#inputs) #output{
                    extern "Rust" {
                        fn #api_name ( #inputs ) #output;
                    }

                    #api_name ( #(#args)* )
                }
            });
        }
    }

    quote! {
        #f
        #(#funcs)*

    }
    .into()
}

#[proc_macro_attribute]
pub fn api_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as ItemImpl);

    let mut funcs = Vec::new();

    let ty = f.self_ty.clone();

    for item in &f.items {
        if let ImplItem::Fn(func) = item {
            let ident = func.sig.ident.clone();
            let inputs = func.sig.inputs.clone();
            let output = func.sig.output.clone();

            let api_name = func_ident(
                f.trait_.clone().unwrap().1.get_ident().unwrap(),
                &func.sig.ident,
            );

            let mut args = Vec::new();

            for arg in &inputs {
                if let FnArg::Typed(t) = arg {
                    if let Pat::Ident(i) = t.pat.as_ref() {
                        let ident = &i.ident;
                        args.push(quote! { #ident , });
                    }
                }
            }

            funcs.push(quote! {
                #[no_mangle]
                unsafe fn #api_name (#inputs) #output{
                    #ty:: #ident ( #(#args)* )
                }
            });
        }
    }

    quote! {
        #f
        #(#funcs)*

    }
    .into()
}

#[proc_macro]
pub fn build_test_setup(_input: TokenStream) -> TokenStream {
    quote! {
    println!("cargo::rustc-link-arg-tests=-Tlink.x");
    println!("cargo::rustc-link-arg-tests=-no-pie");
    println!("cargo::rustc-link-arg-tests=-znostart-stop-gc");
    }
    .into()
}
