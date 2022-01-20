pub fn main() {

}

pub struct BufChainParams {
    stages: u32,
}

#[derive(Generator)]
pub struct BufChain {
    #[input]
    input: Signal,

    #[output]
    output: Signal,
}

pub struct BufChainPinBuilder {

}



impl BufChain {
    pub fn generate(&self, params: BufChainParams, c: &mut Context) -> BufChain {
        let input = c.signal();
        let mut curr = input;
        for i in 0..self.stages {
            let tmp = c.signal();
            let buf = Buffer::with_params(BufferParams {
                size: 4,
            }).input(curr).output(tmp);
            c.add(buf);
            curr = tmp;
        }

        BufChain {
            input,
            output: curr
        }
    }
}


#[derive(ParametricModule)]
pub struct DelayElement {
    input: Signal,
    output: Signal,
}

impl Generator for DelayElement {
    type Params = DelayElementParams;
    fn generate(params: DelayElementParams, c: &mut Context) {
        let num_stages = 2 * params.delay;
        let input = c.signal();
        let output = c.signal();

        let chain = BufChain::with_params(BufChainParams {
            stages,
        }).input(input).output(output);
        c.add(chain);
    }
}
