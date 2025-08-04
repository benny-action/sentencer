use quick_xml::events::Event;
use quick_xml::{Error, Reader};
use regex::Regex;
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
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

    pub fn interactive_mode(
        &self,
        mut sentences: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if sentences.is_empty() {
            println!("No sentences found in the document.");
            return Ok(());
        }

        let mut current_index = 0;
        let total_sentences = sentences.len();
        let mut has_changes = false;

        self.clear_screen();
        self.show_instructions();

        loop {
            self.display_sentence(
                &sentences[current_index],
                current_index + 1,
                total_sentences,
            );

            let input = self.get_user_input()?;

            match input.as_str() {
                "n" | "next" | "" => {
                    if current_index < total_sentences - 1 {
                        current_index += 1;
                        self.clear_screen();
                    } else {
                        self.clear_screen();
                        println!("You've reached the end of the document");
                        println!("Press 'p' to go back or 'q' to quit.");
                    }
                }
                "p" | "prev" | "previous" => {
                    if current_index > 0 {
                        current_index -= 1;
                        self.clear_screen();
                    } else {
                        self.clear_screen();
                        println!("You're at the beginning of the document.");
                        println!("Press 'n' or 'Enter' to proceed or 'q' to quit.");
                    }
                }
                "e" | "edit" => {
                    let new_sentence = self.edit_sentence(&sentences[current_index])?;
                    if new_sentence != sentences[current_index] {
                        sentences[current_index] = new_sentence;
                        has_changes = true;
                        println!("Sentence updated!");
                        println!("Press any key to continue...");
                        self.get_user_input().ok();
                    }
                    self.clear_screen();
                }
                "f" | "first" => {
                    current_index = 0;
                    self.clear_screen();
                }
                "l" | "last" => {
                    current_index = total_sentences - 1;
                    self.clear_screen();
                }
                "h" | "help" => {
                    self.clear_screen();
                    self.show_instructions();
                }
                "q" | "quit" => {
                    println!("Gooooodbye...");
                    break;
                }
                num_str if num_str.chars().all(|c| c.is_ascii_digit()) => {
                    if let Ok(sentence_num) = num_str.parse::<usize>() {
                        if sentence_num > 0 && sentence_num <= total_sentences {
                            current_index = sentence_num - 1;
                            self.clear_screen();
                        } else {
                            println!(
                                "Invalid sentence number. Must be between 1 and {}.",
                                total_sentences
                            );
                        }
                    }
                }
                _ => {
                    println!("Unknow command: {}. Type 'h' for help.", input);
                }
            }
        }

        Ok(())
    }

    fn clear_screen(&self) {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().unwrap();
    }

    fn show_instructions(&self) {
        println!("ODT Navigator");
        println!("==========================");
        println!();
        println!("Commands:");
        println!(" Enter/n/next -> Next sentence");
        println!(" p/prev       -> Prev sentence");
        println!(" f/first      -> Go to first sentence");
        println!(" l/last       -> Go to last sentence");
        println!(" [number]     -> Jump to sentence number");
        println!(" h/help       -> Show this help...");
        println!(" q/quit       -> Quit");
        println!();
        println!("Press Enter to start...");

        self.get_user_input().ok();
        self.clear_screen();
    }

    fn display_sentence(&self, sentence: &str, current: usize, total: usize) {
        println!("ODT Navigator");
        println!("==========================");
        println!();
        println!("Sentence {} of {}", current, total);
        println!(
            "Progress: [{}{}] {:.1}%",
            "█".repeat(current * 30 / total),
            "░".repeat(30 - (current * 30 / total)),
            (current as f64 / total as f64) * 100.0
        );
        println!();
        println!("┌─────────────────────────────────────────────────────────────┐");

        let wrapped_lines = self.wrap_text(sentence, 59);
        for line in wrapped_lines {
            println!("| {:<59} |", line);
        }

        println!("└─────────────────────────────────────────────────────────────┘");
        println!();
        println!("Command (Enter=next, p=prev, h=help, q=quit)");
        io::stdout().flush().unwrap();
    }

    fn wrap_text(&self, text: &str, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.len() + word.len() + 1 <= width {
                if !current_line.is_empty() {
                    current_line.push(' ');
                }
                current_line.push_str(word);
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                    current_line = String::new();
                }
                if word.len() <= width {
                    current_line.push_str(word);
                } else {
                    //break long words
                    lines.push(word[..width].to_string());
                    if word.len() > width {
                        current_line = word[width..].to_string();
                    }
                }
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        lines
    }

    fn get_user_input(&self) -> Result<String, Box<dyn std::error::Error>> {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_lowercase())
    }

    fn edit_sentence(&self, current_sentence: &str) -> Result<String, Box<dyn std::error::Error>> {
        println!("Edit Mode:");
        println!("====================");
        println!();
        println!("Current sentence:");
        println!("┌─────────────────────────────────────────────────────────────┐");

        let wrapped_lines = self.wrap_text(current_sentence, 59);
        for line in wrapped_lines {
            println!("| {:<59} |", line);
        }

        println!("└─────────────────────────────────────────────────────────────┘");
        println!();
        println!("Enter new text (or press Enter to keep unchanged): ");
        println!("Note type 'cancel' to abort editing");
        println!("> ");
        io::stdout().flush().unwrap();

        let mut new_text = String::new();
        io::stdin().read_line(&mut new_text)?;
        let new_text = new_text.trim();

        if new_text.is_empty() {
            Ok(current_sentence.to_string())
        } else if new_text.to_lowercase() == "cancel" {
            println!("Editing cancelled...");
            Ok(current_sentence.to_string())
        } else {
            Ok(new_text.to_string())
        }
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
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <odt_file>", args[0]);
        eprintln!("Example: {} document.odt", args[0]);
        std::process::exit(1);
    }

    let file_path = &args[1];
    let parser = OdtParser::new()?;

    println!("Parsing ODT file: {}", file_path);
    println!("Please wait... \n");

    match parser.parse_file(file_path) {
        Ok(sentences) => {
            if sentences.is_empty() {
                println!("No sentences found in the document.");
                return Ok(());
            }

            println!("Sucessfully parsed {} sentences!", sentences.len());
            println!("Starting interactive mode... \n");

            parser.interactive_mode(sentences)?;
        }
        Err(e) => {
            eprintln!("Error parsing file: '{}': {}", file_path, e);
            eprintln!("Troubleshooting: File exist? Valid Format? Permissions? Corrupted File?");
            std::process::exit(1);
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
