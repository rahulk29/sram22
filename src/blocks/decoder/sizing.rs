use std::fmt::Debug;

pub trait Tree: Sized {
    fn children(&self) -> &[Self];
    fn children_mut(&mut self) -> &mut [Self];

    fn add_right_child(&mut self, child: Self);
}

pub trait ValueTree<V>: Tree {
    fn value_for_child(&self, idx: usize) -> V;
}

pub fn path_map_tree<T: Tree + Debug, U: ValueTree<V> + Debug, F, V>(
    tree: &T,
    map: &F,
    end: &V,
) -> U
where
    F: Fn(&[&T], &V) -> U,
{
    let path = left_path(tree);
    let mut mapped_path = map(&path, end);
    assert_eq!(left_path_len(&mapped_path), path.len());

    let mut state = Some((&mut mapped_path, path[0]));
    while let Some((out, input)) = state {
        for (i, tree) in input.children().iter().enumerate().skip(1) {
            let subtree = path_map_tree(tree, map, &out.value_for_child(i));
            out.add_right_child(subtree);
        }
        state = input
            .children()
            .first()
            .map(|child| (&mut out.children_mut()[0], child));
    }

    mapped_path
}

fn left_path<T: Tree>(tree: &T) -> Vec<&T> {
    let mut nodes = Vec::new();
    let mut root = Some(tree);
    while let Some(node) = root {
        nodes.push(node);
        root = node.children().first();
    }
    nodes
}

fn left_path_len<T: Tree>(tree: &T) -> usize {
    let mut nodes = Vec::new();
    let mut root = Some(tree);
    let mut count = 0;
    while let Some(node) = root {
        nodes.push(node);
        root = node.children().first();
        count += 1;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, Eq, PartialEq)]
    struct TreeNode {
        children: Vec<TreeNode>,
    }

    #[derive(Debug, Default, Eq, PartialEq)]
    struct ValueTreeNode {
        value: i32,
        children: Vec<ValueTreeNode>,
    }

    impl Tree for TreeNode {
        fn children(&self) -> &[Self] {
            &self.children
        }

        fn children_mut(&mut self) -> &mut [Self] {
            &mut self.children
        }

        fn add_right_child(&mut self, child: Self) {
            self.children.push(child)
        }
    }

    impl Tree for ValueTreeNode {
        fn children(&self) -> &[Self] {
            &self.children
        }

        fn children_mut(&mut self) -> &mut [Self] {
            &mut self.children
        }

        fn add_right_child(&mut self, child: Self) {
            self.children.push(child)
        }
    }

    impl ValueTreeNode {
        pub fn new(value: i32) -> Self {
            Self {
                children: vec![],
                value,
            }
        }
    }

    impl ValueTree<i32> for ValueTreeNode {
        fn value_for_child(&self, _: usize) -> i32 {
            self.value
        }
    }

    fn map(input: &[&TreeNode], value: &i32) -> ValueTreeNode {
        assert!(!input.is_empty());

        let mut value = *value;
        value *= 2;

        let mut tree = ValueTreeNode {
            children: vec![],
            value,
        };

        let mut root = &mut tree;
        for _ in &input[1..] {
            value *= 2;
            root.add_right_child(ValueTreeNode {
                children: vec![],
                value,
            });
            root = &mut root.children_mut()[0];
        }

        tree
    }

    #[test]
    fn test_path_map_tree_simple() {
        let input = TreeNode {
            children: vec![TreeNode::default(), TreeNode::default()],
        };

        let output = ValueTreeNode {
            value: 2,
            children: vec![ValueTreeNode::new(4), ValueTreeNode::new(4)],
        };

        let tree = path_map_tree(&input, &map, &1);
        assert_eq!(tree, output);
    }
    #[test]
    fn test_path_map_tree() {
        let input = TreeNode {
            children: vec![
                TreeNode {
                    children: vec![TreeNode::default(), TreeNode::default()],
                },
                TreeNode {
                    children: vec![
                        TreeNode::default(),
                        TreeNode::default(),
                        TreeNode::default(),
                    ],
                },
                TreeNode::default(),
            ],
        };

        let output = ValueTreeNode {
            value: 2,
            children: vec![
                ValueTreeNode {
                    value: 4,
                    children: vec![ValueTreeNode::new(8), ValueTreeNode::new(8)],
                },
                ValueTreeNode {
                    value: 4,
                    children: vec![
                        ValueTreeNode::new(8),
                        ValueTreeNode::new(8),
                        ValueTreeNode::new(8),
                    ],
                },
                ValueTreeNode::new(4),
            ],
        };

        let tree = path_map_tree(&input, &map, &1);
        assert_eq!(tree, output);
    }
}
