use crate::context::ContextTree;

pub fn print_tree(t: &ContextTree) -> &ContextTree {
    print_tree_recur(t, 0);
    t
}

fn print_tree_recur(t: &ContextTree, level: usize) {
    print_indents(level);

    println!("{}", t.module.name());
    for r in t.ctx.resistors.iter() {
        print_indents(level + 1);
        println!("Resistor: {}", r.value());
    }

    for m in &t.children {
        print_tree_recur(m, level + 1);
    }
}

fn print_indents(level: usize) {
    for _ in 0..level {
        print!("-");
    }
}
