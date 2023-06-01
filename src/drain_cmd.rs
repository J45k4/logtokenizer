use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::time::Instant;

use crate::args::DrainArgs;
use crate::drain::DrainParser;
use crate::utility::create_file_for_append;


pub fn drain_cmd(args: DrainArgs) {
    let mut file = File::open(&args.input_path).unwrap();

    let mut logtext = String::new();
    
    file.read_to_string(&mut logtext).unwrap();

    let mut drain_parser = DrainParser::new();

    let mut templates = Vec::new();
    let mut tokens = Vec::new();

    let mut csv_writer = match args.save_csv {
        Some(save_csv) => {
            let mut writer = csv::WriterBuilder::new()
            .delimiter(b';')
            .from_path(save_csv).unwrap();  

            let mut record = csv::StringRecord::new();

            record.push_field("timestamp");
            record.push_field("template");
            record.push_field("label");

            writer.write_record(&record).unwrap();

            Some(writer)
        },
        None => None,
    };
        
    let lines_count = logtext.lines().count();

    let timer = Instant::now();

    for (line_num, line) in logtext.lines().enumerate() {
        let parser_output = drain_parser.parse(&line).unwrap();

        templates.push(parser_output.template);
        tokens.push(parser_output.tokens.join(" "));

        if let Some(csv_writer) = &mut csv_writer {
            let mut record = csv::StringRecord::new();

            record.push_field("");
            record.push_field(&parser_output.template.to_string());
            record.push_field("");

            csv_writer.write_record(&record).unwrap();
        }

        if line_num % 10_000 == 0 {
            let elapsed_seconds = timer.elapsed().as_secs_f64();
            let speed = line_num as f64 / elapsed_seconds;

            println!("[{}/{}] {:.2} lines/s", line_num, lines_count, speed);
        }
    }

    if let Some(save_tokens_path) = args.save_tokens {
        let mut output_file = create_file_for_append(save_tokens_path);

        for token in &tokens {
            output_file.write_all(format!("{}\n", token).as_bytes()).unwrap();
        }
    }

    if let Some(save_templates_path) = args.save_templates {
        let mut output_file = create_file_for_append(save_templates_path);

        for template in &templates {
            output_file.write_all(format!("{}\n", template).as_bytes()).unwrap();
        }
    }

    let count_templates = drain_parser.count_logtemplates();

    println!("Number of templates: {}", count_templates);
}