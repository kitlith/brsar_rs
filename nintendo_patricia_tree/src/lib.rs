use bitvec::prelude::*;
use binread::BinRead;

// TODO: file read/write code
#[derive(BinRead)]
pub struct PatriciaTree<T: BinRead<Args=()>> {
    root_index: u32,
    #[allow(unused)]
    node_count: u32,
    #[br(count = node_count)]
    nodes: Vec<Node<T>>
}

fn null_ffffffff(input: [u32; 2]) -> [Option<u32>; 2] {
    fn inner(input: u32) -> Option<u32> {
        if input == 0xFFFFFFFF {
            None
        } else {
            Some(input)
        }
    }

    [inner(input[0]), inner(input[1])]
}

#[derive(BinRead)]
struct Node<T: BinRead<Args=()>> {
    #[br(map = |x: u16| x != 0)]
    is_leaf: bool,
    bit_index: u16,
    #[br(map = null_ffffffff)]
    next_index: [Option<u32>; 2],
    data: T
}

impl<T: BinRead<Args=()>> PatriciaTree<T> {
    pub fn search(&self, str: &[u8]) -> Option<&T> {
        let str = str.view_bits::<Msb0>();

        let mut cur_node = self.nodes.get(self.root_index as usize)?;

        while !cur_node.is_leaf {
            let bit = match str.get(cur_node.bit_index as usize) {
                Some(bit) => *bit,
                None => break // end of string is also an exit condition
            };

            let next_index = cur_node.next_index[bit as usize]?;
            cur_node = self.nodes.get(next_index as usize)?;
        }

        (&cur_node.data).into()
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        self.nodes.get(idx).map(|node| &node.data)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
