use log::debug;
use valence_data::Frame;
use valence_graph::{VNode, VNodeCtx, VSink};
use valence_runtime_ffi::{
    prost::Message,
    protos::{
        MixDbCreateCollectionProto, MixDbCreateVectorIndex, MixDbFinishVectorIndex,
        MixDbInsertProto, MixDbInsertVector,
    },
    ByteBuffer,
};

use anyhow::{anyhow, Result};

use crate::ai::EmbeddingModel;

pub trait IntoChunks {
    fn into_chunks(self) -> Vec<String>;
}

impl IntoChunks for String {
    fn into_chunks(self) -> Vec<String> {
        vec![self]
    }
}

impl IntoChunks for Vec<String> {
    fn into_chunks(self) -> Vec<String> {
        self
    }
}

extern "C" {
    /// creates a collection and returns a handle to it
    fn _mixdb_create_coll(cmd: *const ByteBuffer) -> ();

    /// inserts a document into a collection by handle, returns document id
    fn _mixdb_insert(cmd: *const ByteBuffer) -> i64;

    fn _mixdb_create_fts_index(cmd: *const ByteBuffer) -> u32;
    fn _mixdb_insert_fts_index(cmd: *const ByteBuffer) -> u32;

    fn _mixdb_create_vector_index(cmd: *const ByteBuffer) -> ();
    fn _mixdb_insert_vector(cmd: *const ByteBuffer) -> ();
    fn _mixdb_finish_vector_index(cmd: *const ByteBuffer) -> ();

    /// opens an iterator over a collection
    fn _mixdb_coll_iterator(coll_handle: u32) -> usize;

    /// returns the next document in the iterator
    fn _mixdb_coll_iterator_next(iter_handle: u32) -> *const ByteBuffer;

    fn _mxl_embed_data(cmd: *const ByteBuffer) -> *const ByteBuffer;
}

struct MxlVectorConfig {
    chunk_fn: Box<dyn Fn(String) -> Vec<String> + Send + Sync>,
    embedding_model: Box<dyn EmbeddingModel + Send + Sync>,
}

pub struct MxlCollectionSink {
    // coll_handle: u32,
    coll_name: String,

    vector_config: Option<MxlVectorConfig>,
    // fts_buf: Option<Arc<RwLock<Vec<String>>>>,
}

impl MxlCollectionSink {
    pub fn new(name: &str) -> Self {
        debug!("creating collection {}", name);

        let create_proto = MixDbCreateCollectionProto {
            db_name: "default".to_owned(),
            collection: name.to_owned(),
            element_type: "Content".to_owned(),
            id_field: "id".to_owned(),
        };

        let create_buf: ByteBuffer = create_proto.encode_to_vec().into();
        let _coll_handle = unsafe { _mixdb_create_coll(&create_buf) };

        Self {
            coll_name: name.to_owned(),
            vector_config: None,
        }
    }

    pub fn vector_index<E, F, C>(&mut self, embedding_model: E, chunk_fn: F) -> Result<()>
    where
        E: EmbeddingModel + Send + Sync + 'static,
        C: IntoChunks,
        F: Fn(String) -> C + Send + Sync + 'static,
    {
        if self.vector_config.is_some() {
            return Err(anyhow!("collection already has vector index"));
        }

        let create_vec_idx = MixDbCreateVectorIndex {
            db_name: "default".to_owned(),
            collection: self.coll_name.clone(),
            dimensions: embedding_model.num_dims() as i32,
        };

        let create_buf: ByteBuffer = create_vec_idx.encode_to_vec().into();
        let _coll_handle = unsafe { _mixdb_create_vector_index(&create_buf) };

        let chunk_fn = move |chunk: String| {
            let chunks = (chunk_fn)(chunk).into_chunks();
            chunks
        };

        self.vector_config = Some(MxlVectorConfig {
            chunk_fn: Box::new(chunk_fn),
            embedding_model: Box::new(embedding_model),
        });

        Ok(())
    }

    fn index_frame(&self, doc_id: u32, document: String) -> Result<()> {
        if let Some(vector_config) = self.vector_config.as_ref() {
            let chunks = (vector_config.chunk_fn)(document);

            for chunk in chunks {
                let embedding = vector_config.embedding_model.embed(&chunk)?;

                let insert_proto = MixDbInsertVector {
                    collection: self.coll_name.clone(),
                    index_name: "default".to_owned(),
                    document_id: doc_id as i32,
                    chunk_text: chunk,
                    vector: embedding,
                };

                let insert_buf: ByteBuffer = insert_proto.encode_to_vec().into();

                unsafe { _mixdb_insert_vector(&insert_buf) };
            }
        }

        Ok(())
    }

    fn finish_indexes(&mut self) -> Result<()> {
        if let Some(_) = self.vector_config.as_ref() {
            let finish_proto = MixDbFinishVectorIndex {
                collection: self.coll_name.clone(),
            };

            let finish_buf: ByteBuffer = finish_proto.encode_to_vec().into();
            unsafe { _mixdb_finish_vector_index(&finish_buf) };
            self.vector_config = None;
        }

        Ok(())
    }
}

impl VNode for MxlCollectionSink {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> () {
        let next = self.recv(ctx);

        match &next {
            Some(Frame::Data(data)) => {
                let insert_proto = MixDbInsertProto {
                    db_name: "default".to_owned(),
                    collection: self.coll_name.clone(),
                    json: data.clone(),
                };

                let insert_buf: ByteBuffer = insert_proto.encode_to_vec().into();
                let doc_id = unsafe { _mixdb_insert(&insert_buf) };

                debug!(
                    "inserted document {} into collection {}",
                    doc_id, self.coll_name
                );

                self.index_frame(doc_id as u32, data.clone()).unwrap();
            }
            // Some(Frame::End) => {

            // }
            _ => (),
        }
    }
}

// TODO implement this in a better way. we were previously hooking on Frame::End but that
// breaks if there are several upstream edges to the sink. we need to hook on the last
// ideally the runtime will notify us that there's no more data coming
impl Drop for MxlCollectionSink {
    fn drop(&mut self) {
        self.finish_indexes().unwrap();
    }
}

impl VSink for MxlCollectionSink {
    type Input = String;
}
