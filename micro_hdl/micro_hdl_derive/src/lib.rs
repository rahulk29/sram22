extern crate proc_macro;

use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DataStruct, DeriveInput, Field, Fields, Ident};

enum DerivePinType {
    Input,
    Output,
    InOut,
}
struct AbstractPort {
    name: Ident,
    ptype: DerivePinType,
    is_wire: bool,
}

#[proc_macro_attribute]
pub fn module(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let inst_name = format!("{}Instance", name);
    let inst_ident = Ident::new(&inst_name, name.span());

    let inst_builder_name = format!("{}InstanceBuilder", name);
    let inst_builder_ident = Ident::new(&inst_builder_name, name.span());

    let fields = if let Data::Struct(DataStruct {
        fields: Fields::Named(fields),
        ..
    }) = ast.data
    {
        fields.named
    } else {
        unimplemented!()
    };

    let params_vec = fields
        .iter()
        .filter(|f| f.attrs.iter().any(is_params))
        .collect::<Vec<_>>();

    let (has_params, params_name, params_ty) = if params_vec.is_empty() {
        (false, None, None)
    } else {
        assert_eq!(params_vec.len(), 1);
        let param = params_vec[0];
        (
            true,
            Some(param.ident.as_ref().unwrap()),
            Some(param.ty.clone()),
        )
    };

    let mut ports = vec![];

    let input_fields = fields
        .iter()
        .filter(|f| f.attrs.iter().any(is_input))
        .collect::<Vec<_>>();
    for f in input_fields.iter() {
        ports.push(AbstractPort {
            name: f.ident.clone().unwrap(),
            ptype: DerivePinType::Input,
            is_wire: is_wire(f),
        });
    }

    let inout_fields = fields
        .iter()
        .filter(|f| f.attrs.iter().any(is_inout))
        .collect::<Vec<_>>();
    for f in inout_fields.iter() {
        ports.push(AbstractPort {
            name: f.ident.clone().unwrap(),
            ptype: DerivePinType::InOut,
            is_wire: is_wire(f),
        });
    }

    let output_fields = fields
        .iter()
        .filter(|f| f.attrs.iter().any(is_output))
        .collect::<Vec<_>>();
    for f in output_fields.iter() {
        ports.push(AbstractPort {
            name: f.ident.clone().unwrap(),
            ptype: DerivePinType::Output,
            is_wire: is_wire(f),
        });
    }

    let generate_return = ports
        .iter()
        .map(|port| {
            let id = port.name.clone();
            if port.is_wire {
                quote! {
                    micro_hdl::Signal::Wire(instance.#id)
                }
            } else {
                quote! {
                    micro_hdl::Signal::Bus(instance.#id)
                }
            }
        })
        .collect::<Vec<_>>();

    let ports_return = ports
        .iter()
        .map(|port| {
            let name = port.name.clone();
            let pin_type = match port.ptype {
                DerivePinType::Input => quote! { pin_type: micro_hdl::PinType::Input },
                DerivePinType::Output => quote! { pin_type: micro_hdl::PinType::Output },
                DerivePinType::InOut => quote! { pin_type: micro_hdl::PinType::InOut },
            };
            let signal = if port.is_wire {
                quote! { signal: micro_hdl::Signal::Wire(self.#name) }
            } else {
                quote! { signal: micro_hdl::Signal::Bus(self.#name.clone()) }
            };

            quote! {
                micro_hdl::Port {
                    name: stringify!(#name).to_string(),
                    #pin_type,
                    #signal,
                }
            }
        })
        .collect::<Vec<_>>();

    let generate_instance = if has_params {
        let param_name = params_name.unwrap();
        quote! {
            let instance = #name::generate(self.#param_name, c);
        }
    } else {
        quote! {
            let instance = #name::generate(c);
        }
    };

    let name_instance = if has_params {
        let param_name = params_name.unwrap();
        quote! { #name::name(self.#param_name) }
    } else {
        quote! { #name::name() }
    };

    let generate_impl = quote! {
        fn generate(&self, c: &mut micro_hdl::context::Context) -> Vec<micro_hdl::Signal> {
            #generate_instance
            vec![#(#generate_return,)*]
        }
    };

    let mut all_fields = input_fields.clone();
    all_fields.append(&mut inout_fields.clone());
    all_fields.append(&mut output_fields.clone());
    let all_fields = all_fields;

    let mut inst_fields = all_fields
        .iter()
        .map(|f| {
            let id = f.ident.as_ref().unwrap();
            let ty = &f.ty;
            quote! {
                #id: #ty
            }
        })
        .collect::<Vec<_>>();

    let mut inst_builder_fields = all_fields
        .iter()
        .map(|f| {
            let id = f.ident.as_ref().unwrap();
            let ty = &f.ty;
            quote! {
                #id: std::option::Option<#ty>
            }
        })
        .collect::<Vec<_>>();

    let mut inst_builder_empty = all_fields
        .iter()
        .map(|f| {
            let id = f.ident.as_ref().unwrap();
            quote! {
                #id: std::option::Option::None
            }
        })
        .collect::<Vec<_>>();

    let mut builder_setters = all_fields
        .iter()
        .map(|f| {
            let id = f.ident.as_ref().unwrap();
            let ty = &f.ty;
            quote! {
                pub fn #id(mut self, #id: #ty) -> Self {
                    self.#id = std::option::Option::Some(#id);
                    self
                }
            }
        })
        .collect::<Vec<_>>();

    let mut build_fields = all_fields
        .iter()
        .map(|f| {
            let id = f.ident.as_ref().unwrap();
            quote! {
                #id: self.#id.unwrap()
            }
        })
        .collect::<Vec<_>>();

    if has_params {
        let param_name = params_name.unwrap();
        let param_ty = params_ty.unwrap();

        inst_fields.push(quote! {
            #param_name: #param_ty
        });

        inst_builder_fields.push(quote! {
            #param_name: std::option::Option<#param_ty>
        });

        inst_builder_empty.push(quote! {
            #param_name: std::option::Option::None
        });

        builder_setters.push(quote! {
            pub fn #param_name(mut self, #param_name: #param_ty) -> Self {
                self.#param_name = std::option::Option::Some(#param_name);
                self
            }
        });

        build_fields.push(quote! {
            #param_name: self.#param_name.unwrap()
        });
    }

    let result = quote! {
        pub struct #name {
        }

        #[must_use = "creating a module instance has no effect; you must add it to a Context"]
        pub struct #inst_ident {
            #(#inst_fields,)*
        }

        #[must_use = "creating a module instance has no effect; you must add it to a Context"]
        pub struct #inst_builder_ident {
            #(#inst_builder_fields,)*
        }

        impl #inst_builder_ident {
            #(#builder_setters)*

            pub fn build(self) -> #inst_ident {
                #inst_ident {
                    #(#build_fields,)*
                }
            }
        }

        impl #name {
            pub fn instance() -> #inst_builder_ident {
                #inst_builder_ident {
                    #(#inst_builder_empty,)*
                }
            }
        }

        impl micro_hdl::Module for #inst_ident {}

        impl micro_hdl::ModuleInstance for #inst_ident {
            #generate_impl

            fn spice(&self) -> String {
                unimplemented!()
            }

            fn name(&self) -> String {
                #name_instance
            }

            fn get_ports(&self) -> Vec<micro_hdl::Port> {
                vec![
                    #(#ports_return,)*
                ]
            }

            fn config(&self) -> micro_hdl::ModuleConfig {
                micro_hdl::ModuleConfig::Generate
            }
        }
    };

    result.into()
}

fn is_input(a: &Attribute) -> bool {
    let x = a.path.segments.iter().collect::<Vec<_>>();
    if x.len() != 1 {
        return false;
    }

    x[0].ident.to_string().as_str() == "input"
}

fn is_inout(a: &Attribute) -> bool {
    let x = a.path.segments.iter().collect::<Vec<_>>();
    if x.len() != 1 {
        return false;
    }

    x[0].ident.to_string().as_str() == "inout"
}

fn is_output(a: &Attribute) -> bool {
    let x = a.path.segments.iter().collect::<Vec<_>>();
    if x.len() != 1 {
        return false;
    }

    x[0].ident.to_string().as_str() == "output"
}

fn is_params(a: &Attribute) -> bool {
    let x = a.path.segments.iter().collect::<Vec<_>>();
    if x.len() != 1 {
        return false;
    }

    x[0].ident.to_string().as_str() == "params"
}

fn is_wire(f: &Field) -> bool {
    if let syn::Type::Path(p) = &f.ty {
        let p = p.path.segments.iter().collect::<Vec<_>>();
        p.last().unwrap().ident == "Node"
    } else {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
