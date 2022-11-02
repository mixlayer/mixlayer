use valence::graph;

#[no_mangle]
extern "C" fn _valence_init() -> () {
    let mut g = graph::VGraph::new();

    let names = g.source(graph::source::vec_source(vec![
        "Zack".to_owned(),
        "Coco".to_owned(),
    ]));

    let cap_names = names.map(&mut g, |name| name.to_uppercase());

    let _sink = cap_names.sink(&mut g, graph::sink::DebugSink::new());
}
