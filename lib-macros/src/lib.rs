use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::ItemFn;

#[proc_macro_attribute]
pub fn builder(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item2: proc_macro2::TokenStream = item.clone().into();

    match syn::parse2::<ItemFn>(item2.clone()) {
        Ok(it) => {
            // println!("it: name = {}", it.sig.ident.to_string());

            if it.sig.ident.to_string() != "main" {
                let msg = "valence::main must apply to a function named main";
                let error = syn::Error::new_spanned(&it.sig.ident, msg);
                return token_stream_with_error(item2, error).into();
            }

            match it.sig.output {
                syn::ReturnType::Type(_, ty) if is_mxlgraph(ty.as_ref()) => {}
                _ => {
                    let msg = "mixlayer::main must return a Result<MxlGraph>";
                    let error = syn::Error::new_spanned(&it.sig.ident, msg);
                    return token_stream_with_error(item2, error).into();
                }
            }

            //TODO just unwrapping Result<VGraph> for now, but return actual error to user in future
            let tokens = quote! {
                #[no_mangle]
                extern "C" fn _valence_app_init() -> *mut MxlGraph {
                    #item2

                    let g: MxlGraph = main().unwrap();
                    Box::into_raw(Box::new(g))
                }
            };

            // println!("output: \"{}\"", tokens.to_string());

            return tokens.into();
        }
        Err(e) => return token_stream_with_error(item2, e).into(),
    };
}

fn token_stream_with_error(mut tokens: TokenStream, error: syn::Error) -> TokenStream {
    tokens.extend(error.into_compile_error());
    tokens
}

fn is_mxlgraph(ty: &syn::Type) -> bool {
    let str_type = ty.to_token_stream().to_string();

    // dbg!(&str_type);

    //TODO this will break if the valence crate is aliased, tokio gets around this by letting you
    // define the alias in the macro args
    match str_type.as_str() {
        "Result < MxlGraph >" => {}
        "Result < valence :: MxlGraph >" => {}
        "mixlayer :: Result < mixlayer :: MxlGraph >" => {}
        "mixlayer :: Result < MxlGraph >" => {}
        _ => return false,
    };

    true
}
