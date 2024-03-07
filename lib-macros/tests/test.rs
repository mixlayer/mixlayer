use mixlayer::{MxlGraph, Result};
use mixlayer_macros::builder;

#[test]
fn test() {
    #[builder]
    fn main() -> Result<MxlGraph> {
        Ok(MxlGraph::new())
    }
}
