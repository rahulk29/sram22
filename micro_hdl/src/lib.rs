pub struct Context {
    net_id: u64,
}

pub struct AbstractNet {

}

impl Context {
    pub fn input() -> AbstractNet {

    }

    pub fn output() -> AbstractNet {

    }
}

pub trait Module {
    fn base_name() -> String;
}

pub trait Generator {
    type Params;
    fn generate(params: Self::Params, c: &mut Context);
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
