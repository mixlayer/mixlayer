use valence::Result;
use valence::VGraph;
use valence_macros::builder;

#[test]
fn test() {
    #[builder]
    fn main() -> Result<valence::VGraph> {
        Ok(VGraph::new())
    }
}
