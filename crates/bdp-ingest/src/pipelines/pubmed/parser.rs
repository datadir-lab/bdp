use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;

#[derive(Debug, Default)]
pub struct PubmedArticle {
    pub pmid: i32,
    pub pmcid: Option<String>,
    pub doi: Option<String>,
    pub title: String,
    pub abstract_text: Option<String>,
    pub pub_year: Option<i32>,
    pub journal: Option<String>,
    pub mesh_headings: Vec<MeshHeading>,
    pub authors: Vec<Author>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Default)]
pub struct MeshHeading {
    pub ui: String,
    pub descriptor: String,
    pub is_major_topic: bool,
}

#[derive(Debug, Default)]
pub struct Author {
    pub last_name: Option<String>,
    pub fore_name: Option<String>,
    pub collective: Option<String>,
    pub affiliation: Option<String>,
}

pub fn parse_pubmed_xml(data: &[u8]) -> Result<Vec<PubmedArticle>> {
    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut articles = Vec::new();
    let mut current: Option<PubmedArticle> = None;
    let mut buf = Vec::new();
    let mut abstract_parts: Vec<String> = Vec::new();

    // State tracking
    let mut in_pmid = false;
    let mut in_title = false;
    let mut in_abstract = false;
    let mut in_journal_title = false;
    let mut in_article_id = false;
    let mut article_id_type = String::new();
    let mut in_mesh_descriptor = false;
    let mut mesh_major = false;
    let mut current_mesh: MeshHeading = MeshHeading::default();
    let mut current_author: Option<Author> = None;
    let mut in_last_name = false;
    let mut in_fore_name = false;
    let mut in_collective = false;
    let mut in_affiliation = false;
    let mut in_keyword_list = false;
    let mut in_keyword = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name().as_ref().to_vec();
                match name.as_slice() {
                    b"PubmedArticle" => {
                        current = Some(PubmedArticle::default());
                        abstract_parts.clear();
                    }
                    b"PMID" => in_pmid = true,
                    b"ArticleTitle" => in_title = true,
                    b"AbstractText" => in_abstract = true,
                    b"Title" => in_journal_title = true,
                    b"ArticleId" => {
                        in_article_id = true;
                        article_id_type = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|a| a.key.as_ref() == b"IdType")
                            .map(|a| String::from_utf8_lossy(&a.value).to_string())
                            .unwrap_or_default();
                    }
                    b"DescriptorName" => {
                        in_mesh_descriptor = true;
                        mesh_major = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|a| a.key.as_ref() == b"MajorTopicYN")
                            .map(|a| a.value.as_ref() == b"Y")
                            .unwrap_or(false);
                        current_mesh = MeshHeading::default();
                        current_mesh.is_major_topic = mesh_major;
                        if let Some(ui_attr) = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|a| a.key.as_ref() == b"UI")
                        {
                            current_mesh.ui =
                                String::from_utf8_lossy(&ui_attr.value).to_string();
                        }
                    }
                    b"Author" => current_author = Some(Author::default()),
                    b"LastName" if current_author.is_some() => in_last_name = true,
                    b"ForeName" if current_author.is_some() => in_fore_name = true,
                    b"CollectiveName" if current_author.is_some() => in_collective = true,
                    b"Affiliation" if current_author.is_some() => in_affiliation = true,
                    b"KeywordList" => in_keyword_list = true,
                    b"Keyword" if in_keyword_list => in_keyword = true,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                let name = e.name().as_ref().to_vec();
                match name.as_slice() {
                    b"PubmedArticle" => {
                        if let Some(mut art) = current.take() {
                            if !abstract_parts.is_empty() {
                                art.abstract_text = Some(abstract_parts.join(" "));
                                abstract_parts.clear();
                            }
                            if art.pmid > 0 && !art.title.is_empty() {
                                articles.push(art);
                            }
                        }
                    }
                    b"PMID" => in_pmid = false,
                    b"ArticleTitle" => in_title = false,
                    b"AbstractText" => in_abstract = false,
                    b"Title" => in_journal_title = false,
                    b"ArticleId" => in_article_id = false,
                    b"KeywordList" => in_keyword_list = false,
                    b"Keyword" => in_keyword = false,
                    b"DescriptorName" => {
                        if let Some(art) = current.as_mut() {
                            art.mesh_headings.push(std::mem::take(&mut current_mesh));
                        }
                        in_mesh_descriptor = false;
                    }
                    b"Author" => {
                        if let (Some(auth), Some(art)) =
                            (current_author.take(), current.as_mut())
                        {
                            art.authors.push(auth);
                        }
                        in_last_name = false;
                        in_fore_name = false;
                        in_collective = false;
                        in_affiliation = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                let text = e.decode().unwrap_or_default().into_owned();
                if in_pmid {
                    if let Some(art) = current.as_mut() {
                        art.pmid = text.parse().unwrap_or(0);
                    }
                    in_pmid = false;
                }
                if in_title {
                    if let Some(art) = current.as_mut() {
                        art.title = text.clone();
                    }
                    in_title = false;
                }
                if in_abstract {
                    abstract_parts.push(text.clone());
                }
                if in_journal_title {
                    if let Some(art) = current.as_mut() {
                        if art.journal.is_none() {
                            art.journal = Some(text.clone());
                        }
                    }
                    in_journal_title = false;
                }
                if in_article_id {
                    if let Some(art) = current.as_mut() {
                        match article_id_type.as_str() {
                            "doi" => art.doi = Some(text.clone()),
                            "pmc" => art.pmcid = Some(text.clone()),
                            _ => {}
                        }
                    }
                }
                if in_mesh_descriptor {
                    current_mesh.descriptor = text.clone();
                }
                if in_keyword {
                    if let Some(art) = current.as_mut() {
                        art.keywords.push(text.clone());
                    }
                    in_keyword = false;
                }
                if let Some(auth) = current_author.as_mut() {
                    if in_last_name {
                        auth.last_name = Some(text.clone());
                        in_last_name = false;
                    }
                    if in_fore_name {
                        auth.fore_name = Some(text.clone());
                        in_fore_name = false;
                    }
                    if in_collective {
                        auth.collective = Some(text.clone());
                        in_collective = false;
                    }
                    if in_affiliation {
                        auth.affiliation = Some(text.clone());
                        in_affiliation = false;
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "XML parse error at position {}: {}",
                    reader.error_position(),
                    e
                ))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(articles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_article() {
        let xml = r#"<PubmedArticleSet>
<PubmedArticle>
  <MedlineCitation><PMID Version="1">12345678</PMID>
    <Article><ArticleTitle>Test Title</ArticleTitle>
      <Abstract><AbstractText>Some abstract text.</AbstractText></Abstract>
    </Article>
  </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;
        let articles = parse_pubmed_xml(xml.as_bytes()).unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].pmid, 12345678);
        assert_eq!(articles[0].title, "Test Title");
        assert_eq!(
            articles[0].abstract_text.as_deref(),
            Some("Some abstract text.")
        );
    }

    #[test]
    fn test_parse_skips_empty_pmid() {
        let xml = r#"<PubmedArticleSet>
<PubmedArticle>
  <MedlineCitation><PMID Version="1">0</PMID>
    <Article><ArticleTitle>No Title</ArticleTitle></Article>
  </MedlineCitation>
</PubmedArticle>
</PubmedArticleSet>"#;
        let articles = parse_pubmed_xml(xml.as_bytes()).unwrap();
        assert_eq!(articles.len(), 0);
    }
}
