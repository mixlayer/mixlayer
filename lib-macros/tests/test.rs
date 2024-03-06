use valence::Result;
use valence::MxlGraph;
use valence_macros::builder;

#[test]
fn test() {
    #[builder]
    fn main() -> Result<valence::MxlGraph> {
        Ok(MxlGraph::new())
    }
}
