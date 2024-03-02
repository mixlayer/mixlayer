use log::debug;
use valence_data::{Frame, JsonObject};
use valence_graph::{VNode, VNodeCtx, VSink};
use valence_runtime_ffi::{
    prost::Message,
    protos::{
        MixDbCreateCollectionProto, MixDbCreateSearchIndex, MixDbCreateVectorIndex,
        MixDbFinishVectorIndex, MixDbInsertProto, MixDbInsertVector, MixDbSearchField,
        MixDbSearchFieldType, MixDbSearchFinishIndex, MixDbSearchIndexDocument,
    },
    ByteBuffer,
};

use anyhow::{anyhow, Context, Result};

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

    fn _mixdb_create_search_index(cmd: *const ByteBuffer) -> *const ByteBuffer;
    fn _mixdb_search_index_insert(cmd: *const ByteBuffer) -> *const ByteBuffer;
    fn _mixdb_search_index_finish(cmd: *const ByteBuffer) -> *const ByteBuffer;
}

struct MxlVectorConfig {
    chunk_fn: Box<dyn Fn(&JsonObject) -> Vec<String> + Send + Sync>,
    embedding_model: Box<dyn EmbeddingModel + Send + Sync>,
}

pub struct MxlCollectionSink {
    // coll_handle: u32,
    coll_name: String,

    vector_config: Option<MxlVectorConfig>, //TODO support multiple vector indexes
    search_index: bool,                     //TODO support multiple search indexes
}

impl MxlCollectionSink {
    pub fn new(name: &str, element_type: &str, id_field: &str) -> Self {
        debug!("creating collection {}", name);

        let create_proto = MixDbCreateCollectionProto {
            db_name: "default".to_owned(),
            collection: name.to_owned(),
            element_type: element_type.to_owned(),
            id_field: id_field.to_owned(),
        };

        let create_buf: ByteBuffer = create_proto.encode_to_vec().into();
        let _coll_handle = unsafe { _mixdb_create_coll(&create_buf) };

        Self {
            coll_name: name.to_owned(),
            vector_config: None,
            search_index: false,
        }
    }

    pub fn vector_index<E, F, C>(&mut self, embedding_model: E, chunk_fn: F) -> Result<()>
    where
        E: EmbeddingModel + Send + Sync + 'static,
        C: IntoChunks,
        F: Fn(&JsonObject) -> C + Send + Sync + 'static,
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

        let chunk_fn = move |chunk: &JsonObject| {
            let chunks = (chunk_fn)(chunk).into_chunks();
            chunks
        };

        self.vector_config = Some(MxlVectorConfig {
            chunk_fn: Box::new(chunk_fn),
            embedding_model: Box::new(embedding_model),
        });

        Ok(())
    }

    pub fn search_index(&mut self, indexed_fields: &[&str]) -> Result<()> {
        let fields = indexed_fields
            .iter()
            .map(|s| MixDbSearchField {
                field_name: s.to_string(),
                //FIXME get this field type from the element type schema
                field_type: MixDbSearchFieldType::SearchFieldText as i32,
            })
            .collect();

        let create_search_idx = MixDbCreateSearchIndex {
            db_name: "default".to_owned(),
            coll_name: self.coll_name.clone(),
            index_name: "default".to_owned(),
            fields,
        };

        let create_buf: ByteBuffer = create_search_idx.encode_to_vec().into();
        unsafe { _mixdb_create_search_index(&create_buf) };

        self.search_index = true;

        Ok(())
    }

    fn index_frame(&self, doc_id: u32, document: &JsonObject) -> Result<()> {
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

        if self.search_index {
            let insert_proto = MixDbSearchIndexDocument {
                collection: self.coll_name.clone(),
                index_name: "default".to_owned(),
                document_id: doc_id as i32,
                json: serde_json::to_string(document.as_map()).context("error serializing json")?,
            };

            let insert_buf: ByteBuffer = insert_proto.encode_to_vec().into();
            unsafe { _mixdb_search_index_insert(&insert_buf) };
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

        if self.search_index {
            let finish_proto = MixDbSearchFinishIndex {
                collection: self.coll_name.clone(),
                index_name: "default".to_owned(),
            };

            let finish_buf: ByteBuffer = finish_proto.encode_to_vec().into();
            unsafe { _mixdb_finish_vector_index(&finish_buf) };
        }

        Ok(())
    }
}

impl VNode for MxlCollectionSink {
    fn tick(&mut self, ctx: &mut VNodeCtx) -> Result<()> {
        let next = self.recv(ctx);

        match &next {
            Some(Frame::Data(data)) => {
                let insert_proto = MixDbInsertProto {
                    db_name: "default".to_owned(),
                    collection: self.coll_name.clone(),
                    json: serde_json::to_string(data.as_map()).unwrap(),
                };

                let insert_buf: ByteBuffer = insert_proto.encode_to_vec().into();
                let doc_id = unsafe { _mixdb_insert(&insert_buf) };

                //TODO make this better when we return better ffi errors
                if doc_id < 0 {
                    return Err(anyhow!("error inserting document"));
                }

                debug!(
                    "inserted document {} into collection {}",
                    doc_id, self.coll_name
                );

                self.index_frame(doc_id as u32, data)
                    .context("error indexing frame")?;
            }
            _ => (),
        }

        if ctx.recv_finished() {
            self.finish_indexes()
                .with_context(|| "error finalizing indexes")?;
        }

        Ok(())
    }

    fn default_label(&self) -> Option<String> {
        Some(format!("Collection {}", self.coll_name))
    }
}

// TODO implement this in a better way. we were previously hooking on Frame::End but that
// breaks if there are several upstream edges to the sink. we need to hook on the last
// ideally the runtime will notify us that there's no more data coming
// FIXME this breaks in a sharded environment because it's dropped in every shard
// impl Drop for MxlCollectionSink {
//     fn drop(&mut self) {
//         self.finish_indexes().unwrap();
//     }
// }

impl VSink for MxlCollectionSink {
    type Input = JsonObject;
}
