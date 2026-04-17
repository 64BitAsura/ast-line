use std::collections::HashMap;
use crate::graph::KnowledgeGraph;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub node_id: String,
    pub score: f64,
    pub snippet: String,
}

#[derive(Debug, Default)]
pub struct BM25Index {
    pub docs: Vec<IndexedDoc>,
    pub avg_len: f64,
    pub df: HashMap<String, usize>,
    pub n: usize,
}

#[derive(Debug)]
pub struct IndexedDoc {
    pub node_id: String,
    pub terms: Vec<String>,
    pub snippet: String,
}

pub fn index_graph(graph: &KnowledgeGraph) -> BM25Index {
    let mut docs: Vec<IndexedDoc> = Vec::new();

    for node in graph.nodes() {
        let mut text = node.name.clone();
        if let Some(fp) = node.file_path() {
            text.push(' ');
            text.push_str(fp);
        }
        let snippet = text.clone();
        let terms: Vec<String> = tokenize(&text);
        docs.push(IndexedDoc {
            node_id: node.id.clone(),
            terms,
            snippet,
        });
    }

    let n = docs.len();
    let mut df: HashMap<String, usize> = HashMap::new();
    for doc in &docs {
        let unique: std::collections::HashSet<&String> = doc.terms.iter().collect();
        for term in unique {
            *df.entry(term.clone()).or_insert(0) += 1;
        }
    }

    let avg_len = if n > 0 {
        docs.iter().map(|d| d.terms.len()).sum::<usize>() as f64 / n as f64
    } else {
        1.0
    };

    BM25Index { docs, avg_len, df, n }
}

pub fn search(index: &BM25Index, query: &str, limit: usize) -> Vec<SearchResult> {
    let query_terms = tokenize(query);
    if query_terms.is_empty() || index.n == 0 {
        return vec![];
    }

    let k1 = 1.5f64;
    let b = 0.75f64;

    let mut scores: Vec<(usize, f64)> = index.docs.iter().enumerate().map(|(i, doc)| {
        let doc_len = doc.terms.len() as f64;
        let score: f64 = query_terms.iter().map(|term| {
            let tf = doc.terms.iter().filter(|t| *t == term).count() as f64;
            let df = *index.df.get(term).unwrap_or(&0) as f64;
            if tf == 0.0 || df == 0.0 {
                return 0.0;
            }
            let idf = ((index.n as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
            let tf_norm = tf * (k1 + 1.0) / (tf + k1 * (1.0 - b + b * doc_len / index.avg_len));
            idf * tf_norm
        }).sum();
        (i, score)
    }).collect();

    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(limit);

    scores.into_iter()
        .filter(|(_, s)| *s > 0.0)
        .map(|(i, score)| SearchResult {
            node_id: index.docs[i].node_id.clone(),
            score,
            snippet: index.docs[i].snippet.clone(),
        })
        .collect()
}

fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(String::from)
        .collect()
}
