pub fn main() {

}

pub struct BufChainParams {
    stages: u32,
}

#[derive(Module)]
pub struct BufChain {
    #[input]
    input: Signal,

    #[output]
    output: Signal,
}

impl BufChainGen {
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



#[derive(Module)]
pub struct DelayElement {
    #[input]
    input: Signal,
    #[output]
    output: Signal,
}

impl DelayElement {
    #[generator]
    pub fn generate(params: DelayElementParams, c: &mut Context) -> DelayElement {
        let num_stages = 2 * params.delay;
        let input = c.signal();
        let output = c.signal();

        let chain = BufChain::with_params(BufChainParams {
            stages,
        }).input(input).output(output);
        c.add(chain);

        Self {
            input,
            output,
        }
    }
}
