use quick_xml::events::Event;
use quick_xml::{Error, Reader};
use regex::Regex;
use std::fs::File;
use std::io::{BufReader, Read};
use zip::ZipArchive;

#[derive(Debug)]
pub struct OdtParser {
    sentence_regex: Regex,
}

impl OdtParser {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let sentence_regex = Regex::new(r"[.!?]+\s+")?;

        Ok(OdtParser { sentence_regex })
    }

    pub fn parse_file(&self, file_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)?;

        let mut content_file = archive.by_name("content.xml")?;
        let mut content = String::new();
        content_file.read_to_string(&mut content)?;

        let text = self.extract_text_from_xml(&content)?;

        let sentences = self.split_into_sentences(&text);

        Ok(sentences)
    }

    fn extract_text_from_xml(
        &self,
        xml_content: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut reader = Reader::from_str(xml_content);
        reader.trim_text(true);

        let mut text_content = String::new();
        let mut buf = Vec::new();
        let mut in_text_element = false;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => match e.name().as_ref() {
                    b"text:p" | b"text.span" | b"text.h" => {
                        in_text_element = true;
                    }
                    _ => {}
                },
                Ok(Event::End(ref e)) => match e.name().as_ref() {
                    b"text:p" | b"text.h" => {
                        text_content.push(' ');
                        in_text_element = false;
                    }
                    b"text:span" => {
                        in_text_element = false;
                    }
                    _ => {}
                },
                Ok(Event::Text(e)) => {
                    if in_text_element {
                        let text = e.unescape()?;
                        text_content.push_str(&text);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(format!("Error parsing XML: {}", e).into()),
                _ => {}
            }
            buf.clear();
        }

        Ok(text_content)
    }

    fn split_into_sentences(&self, text: &str) -> Vec<String> {
        let cleaned_text = text.trim().replace('\n', " ");
        let cleaned_text = Regex::new(r"\s+").unwrap().replace_all(&cleaned_text, " ");

        let parts: Vec<&str> = self.sentence_regex.split(&cleaned_text).collect();

        let mut sentences = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                if i < parts.len() - 1 {
                    let next_start =
                        part.as_ptr() as usize + part.len() - cleaned_text.as_ptr() as usize;
                    if let Some(punct_match) =
                        self.sentence_regex.find_at(&cleaned_text, next_start)
                    {
                        let punct = punct_match.as_str().trim();
                        sentences.push(format!("{}{}", trimmed, punct));
                    } else {
                        sentences.push(trimmed.to_string());
                    }
                } else {
                    sentences.push(trimmed.to_string());
                }
            }
        }

        sentences
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = OdtParser::new()?;

    match parser.parse_file("ishmael.odt") {
        Ok(sentences) => {
            println!("Found {} sentences", sentences.len());
            for (i, sentence) in sentences.iter().enumerate() {
                println!("{}. {}", i + 1, sentence);
            }
        }
        Err(e) => {
            eprintln!("Error parsing file: {}", e);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use zip::{ZipWriter, write::FileOptions};

    // Helper function to create a test ODT file
    fn create_test_odt_file(
        file_path: &str,
        content: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(file_path)?;
        let mut zip = ZipWriter::new(file);

        // Create mimetype file (required for ODT)
        zip.start_file(
            "mimetype",
            FileOptions::default().compression_method(zip::CompressionMethod::Stored),
        )?;
        zip.write_all(b"application/vnd.oasis.opendocument.text")?;

        // Create META-INF/manifest.xml (required for ODT)
        zip.start_file("META-INF/manifest.xml", FileOptions::default())?;
        let manifest = r#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0">
    <manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.text"/>
    <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"#;
        zip.write_all(manifest.as_bytes())?;

        // Create content.xml with the test content
        zip.start_file("content.xml", FileOptions::default())?;
        let content_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content 
    xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
    <office:body>
        <office:text>
            {}
        </office:text>
    </office:body>
</office:document-content>"#,
            content
        );
        zip.write_all(content_xml.as_bytes())?;

        zip.finish()?;
        Ok(())
    }

    #[test]
    fn test_odt_file_parsing() -> Result<(), Box<dyn std::error::Error>> {
        let test_file = "test_document.odt";

        // Create test content with multiple paragraphs and sentences
        let test_content = r#"
            <text:p>This is the first paragraph with two sentences. Here is the second sentence!</text:p>
            <text:p>This is a second paragraph. It also has multiple sentences? Yes, it does.</text:p>
            <text:h>This is a heading.</text:h>
            <text:p>Final paragraph after the heading.</text:p>
        "#;

        // Create the test ODT file
        create_test_odt_file(test_file, test_content)?;

        // Parse the file
        let parser = OdtParser::new()?;
        let sentences = parser.parse_file(test_file)?;

        // Clean up test file
        fs::remove_file(test_file).ok();

        // Verify results
        assert!(!sentences.is_empty(), "Should have parsed some sentences");

        // Check that we got the expected sentences
        let expected_phrases = vec![
            "This is the first paragraph with two sentences",
            "Here is the second sentence",
            "This is a second paragraph",
            "It also has multiple sentences",
            "Yes, it does",
            "This is a heading",
            "Final paragraph after the heading",
        ];

        println!("Parsed sentences:");
        for (i, sentence) in sentences.iter().enumerate() {
            println!("  {}: {}", i + 1, sentence);
        }

        // Verify we have at least the expected number of sentences
        assert!(
            sentences.len() >= 6,
            "Should have at least 6 sentences, got {}",
            sentences.len()
        );

        // Check that some expected content is present
        let all_text = sentences.join(" ");
        assert!(
            all_text.contains("first paragraph"),
            "Should contain 'first paragraph'"
        );
        assert!(
            all_text.contains("second sentence"),
            "Should contain 'second sentence'"
        );
        assert!(all_text.contains("heading"), "Should contain 'heading'");

        Ok(())
    }

    #[test]
    fn test_file_not_found() {
        let parser = OdtParser::new().unwrap();
        let result = parser.parse_file("nonexistent.odt");
        assert!(result.is_err(), "Should return error for nonexistent file");
    }

    #[test]
    fn test_invalid_odt_file() -> Result<(), Box<dyn std::error::Error>> {
        let test_file = "invalid.odt";

        // Create a file that's not a valid ODT (missing content.xml)
        let file = File::create(test_file)?;
        let mut zip = ZipWriter::new(file);
        zip.start_file("dummy.txt", FileOptions::default())?;
        zip.write_all(b"This is not an ODT file")?;
        zip.finish()?;

        let parser = OdtParser::new()?;
        let result = parser.parse_file(test_file);

        // Clean up
        fs::remove_file(test_file).ok();

        assert!(result.is_err(), "Should return error for invalid ODT file");
        Ok(())
    }

    #[test]
    fn test_xml_extraction() -> Result<(), Box<dyn std::error::Error>> {
        let parser = OdtParser::new()?;

        // Test XML content extraction directly
        let xml_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content 
    xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
    xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0">
    <office:body>
        <office:text>
            <text:p>First paragraph.</text:p>
            <text:p>Second paragraph with <text:span>inline text</text:span>.</text:p>
        </office:text>
    </office:body>
</office:document-content>"#;

        let extracted_text = parser.extract_text_from_xml(xml_content)?;

        assert!(
            extracted_text.contains("First paragraph"),
            "Should extract first paragraph"
        );
        assert!(
            extracted_text.contains("Second paragraph"),
            "Should extract second paragraph"
        );
        assert!(
            extracted_text.contains("inline text"),
            "Should extract span text"
        );

        println!("Extracted text: '{}'", extracted_text);

        Ok(())
    }

    #[test]
    fn test_sentence_splitting() {
        let parser = OdtParser::new().unwrap();
        let text = "This is sentence one. This is sentence two! And this is sentence three?";
        let sentences = parser.split_into_sentences(text);

        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "This is sentence one.");
        assert_eq!(sentences[1], "This is sentence two!");
        assert_eq!(sentences[2], "And this is sentence three?");
    }

    #[test]
    fn test_empty_text() {
        let parser = OdtParser::new().unwrap();
        let sentences = parser.split_into_sentences("");
        assert_eq!(sentences.len(), 0);
    }

    #[test]
    fn test_complex_punctuation() {
        let parser = OdtParser::new().unwrap();
        let text = "What?! Really... Yes. No doubt about it!!! Absolutely.";
        let sentences = parser.split_into_sentences(text);

        println!("Complex punctuation sentences:");
        for (i, sentence) in sentences.iter().enumerate() {
            println!("  {}: {}", i + 1, sentence);
        }

        assert!(sentences.len() >= 3, "Should handle complex punctuation");
    }
}
