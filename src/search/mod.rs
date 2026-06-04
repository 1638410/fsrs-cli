use anyhow::{Context, Result};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter};

use crate::core::types::Card;

/// Tantivy 全文搜索引擎
pub struct SearchEngine {
    index: Index,
    reader: IndexReader,
    schema: Schema,
}

/// 搜索结果
pub struct SearchResult {
    pub card_id: String,
    pub score: f32,
}

/// 构建 jieba 分词的 TEXT 选项
fn jieba_text() -> TextOptions {
    TextOptions::default().set_stored().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("jieba")
            .set_fieldnorms(true)
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    )
}

impl SearchEngine {
    /// 打开或创建搜索索引
    pub fn open(index_path: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        schema_builder.add_text_field("card_id", STRING | STORED);
        schema_builder.add_text_field("question", jieba_text());
        schema_builder.add_text_field("answer", jieba_text());
        schema_builder.add_text_field("tags", jieba_text());

        let schema = schema_builder.build();

        // 创建或打开索引
        let index = if index_path.exists() {
            Index::open_in_dir(index_path)
                .with_context(|| format!("无法打开索引: {}", index_path.display()))?
        } else {
            std::fs::create_dir_all(index_path)?;
            Index::create_in_dir(index_path, schema.clone())
                .with_context(|| format!("无法创建索引: {}", index_path.display()))?
        };

        // 注册 jieba 分词器
        index.tokenizers().register(
            "jieba",
            <tantivy_jieba::JiebaTokenizer as Default>::default(),
        );

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()
            .context("无法创建索引读取器")?;

        Ok(SearchEngine {
            index,
            reader,
            schema,
        })
    }

    /// 索引一张卡片
    pub fn index_card(&self, card: &Card) -> Result<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(50_000_000)
            .context("无法创建索引写入器")?;

        let card_id = self.schema.get_field("card_id").unwrap();
        let question = self.schema.get_field("question").unwrap();
        let answer = self.schema.get_field("answer").unwrap();
        let tags = self.schema.get_field("tags").unwrap();

        let doc = doc!(
            card_id => card.id.as_str(),
            question => card.question.as_str(),
            answer => card.answer.as_str(),
            tags => card.tags.join(" ").as_str(),
        );

        writer.add_document(doc)?;
        writer.commit()?;

        Ok(())
    }

    /// 批量索引卡片
    pub fn index_cards(&self, cards: &[Card]) -> Result<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(50_000_000)
            .context("无法创建索引写入器")?;

        let card_id = self.schema.get_field("card_id").unwrap();
        let question = self.schema.get_field("question").unwrap();
        let answer = self.schema.get_field("answer").unwrap();
        let tags = self.schema.get_field("tags").unwrap();

        for card in cards {
            let doc = doc!(
                card_id => card.id.as_str(),
                question => card.question.as_str(),
                answer => card.answer.as_str(),
                tags => card.tags.join(" ").as_str(),
            );
            writer.add_document(doc)?;
        }

        writer.commit()?;
        Ok(())
    }

    /// 搜索卡片
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();

        let question = self.schema.get_field("question").unwrap();
        let answer = self.schema.get_field("answer").unwrap();
        let tags = self.schema.get_field("tags").unwrap();

        let query_parser = QueryParser::for_index(&self.index, vec![question, answer, tags]);

        let query = query_parser
            .parse_query(query_str)
            .with_context(|| format!("无法解析搜索查询: {}", query_str))?;

        // tantivy 0.26: TopDocs 需要 .order_by_score() 才能作为 Collector
        let collector = TopDocs::with_limit(limit).order_by_score();
        let top_docs = searcher.search(&query, &collector).context("搜索失败")?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc = searcher.doc::<tantivy::TantivyDocument>(doc_address)?;
            let card_id_field = self.schema.get_field("card_id").unwrap();

            if let Some(card_id_value) = doc.get_first(card_id_field) {
                if let Some(card_id) = card_id_value.as_str() {
                    results.push(SearchResult {
                        card_id: card_id.to_string(),
                        score,
                    });
                }
            }
        }

        Ok(results)
    }

    /// 删除卡片的索引
    pub fn remove_card(&self, card_id: &str) -> Result<()> {
        let mut writer: IndexWriter = self
            .index
            .writer(50_000_000)
            .context("无法创建索引写入器")?;

        let card_id_field = self.schema.get_field("card_id").unwrap();
        let term = Term::from_field_text(card_id_field, card_id);
        writer.delete_term(term);
        writer.commit()?;

        Ok(())
    }
}
