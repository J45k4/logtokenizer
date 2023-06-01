use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use byteorder::LittleEndian;
use byteorder::ReadBytesExt;

const DRAIN_MAGIC: u32 = 0x94067823;
const BEGIN_CLUSTER_GROUP: u16 = 0x01;
const END_CLUSTER_GROUP: u16 = 0x02;
const BEGIN_EVENT_GROUP: u16 = 0x03;
const END_EVENT_GROUP: u16 = 0x04;
const BEGIN_TEMPLATE: u16 = 0x05;
const END_TEMPLATE: u16 = 0x06;
const CURR_DRAIN_MODEL_VERSION: u32 = 1;

type Tokens = Vec<String>;

#[derive(Debug)]
pub struct DrainParseOutput {
    pub template: usize,
    pub tokens: Tokens,
    pub parameters: Vec<String>,
}

pub fn tokenize<'a>(input: &'a str, delimiters: &'a [char]) -> impl Iterator<Item = &'a str> {
    input
        .trim()
        .split(&delimiters[..])
        .filter(|s| !s.is_empty())
}

fn simseq(seq1: &Tokens, seq2: &Tokens) -> usize {
    let mut sum = 0;

    for i in 0..seq1.len() {
        if seq1[i] == seq2[i] {
            sum += 1;
        }
    } 

    sum
}

#[derive(Debug, PartialEq)]
pub struct DrainParser {
    new_id: usize,
    pub parsers: HashMap<usize, HashMap<String, LogCluster>>,
}

impl DrainParser {
    pub fn new() -> Self {
        DrainParser {
            new_id: 1,
            parsers: HashMap::new(),
        }
    }

    pub fn parse(&mut self, input: &str) -> Option<DrainParseOutput> {
        let mut tokens: Tokens = tokenize(input, &[' ', '=', ',', ':'])
            .map(|t| t.to_string())
            .collect();

        let l = tokens.len();

        if l == 0 {
            return None;
        }

        let map = match self.parsers.get_mut(&l) {
            Some(p) => p,
            None => {
                let p = HashMap::new();
                self.parsers.insert(l, p);
                self.parsers.get_mut(&l).unwrap()
            },
        };

        let first_token = &tokens[0];

        let event = if first_token.parse::<f64>().is_ok() {
            "*".to_string()
        } else {
            first_token.to_string()
        };

        let cluster = match map.get_mut(&event) {
            Some(c) => c,
            None => {
                let c = LogCluster::new(l);
                map.insert(event.clone(), c);
                map.get_mut(&event).unwrap()
            },
        };

        let out = cluster.process(tokens, self.new_id);

        if out.template == self.new_id {
            self.new_id += 1;
        }
        
        Some(out)
    }

	pub fn count_logtemplates(&self) -> usize {
		let mut count = 0;
		for (_, cluster_group) in &self.parsers {
			for (_, cluster) in cluster_group {
				count += cluster.templates.len();
			}
		}

		count
	}

	pub fn save<P>(&mut self, path: P)
	where P: AsRef<Path> {
		let mut file = File::create(path).unwrap();

		self.save_writer(&mut file);
	}

	pub fn save_writer(&mut self, writer: &mut impl Write) {
		writer.write(DRAIN_MAGIC.to_le_bytes().as_ref()).unwrap();
		writer.write(CURR_DRAIN_MODEL_VERSION.to_le_bytes().as_ref()).unwrap();
		writer.write((self.new_id as u32).to_le_bytes().as_ref()).unwrap();

		for (len, cluster_group) in &self.parsers {
			writer.write(BEGIN_CLUSTER_GROUP.to_le_bytes().as_ref()).unwrap();
			writer.write((*len as u32).to_le_bytes().as_ref()).unwrap();

			for (event, cluster) in cluster_group {
				writer.write(BEGIN_EVENT_GROUP.to_le_bytes().as_ref()).unwrap();
				let event_len = event.len() as u32;
				writer.write(event_len.to_le_bytes().as_ref()).unwrap();
				writer.write(event.as_bytes()).unwrap();

				for template in &cluster.templates {
					writer.write(BEGIN_TEMPLATE.to_le_bytes().as_ref()).unwrap();

					writer.write((template.id as u32) .to_le_bytes().as_ref()).unwrap();
					writer.write((template.count as u32).to_le_bytes().as_ref()).unwrap();
					writer.write((template.tokens.len() as u32).to_le_bytes().as_ref()).unwrap();

					for token in &template.tokens {
						let len = token.len() as u32;
						writer.write(len.to_le_bytes().as_ref()).unwrap();
						writer.write(token.as_bytes()).unwrap();
					}
				}
				writer.write(END_TEMPLATE.to_le_bytes().as_ref()).unwrap();
			}
			writer.write(END_EVENT_GROUP.to_le_bytes().as_ref()).unwrap();
		}
		writer.write(END_CLUSTER_GROUP.to_le_bytes().as_ref()).unwrap();
	}

	pub fn load<P>(&mut self, path: P)
	where P: AsRef<Path>
	{
		let timer = std::time::Instant::now();

		let path = path.as_ref();

		println!("Loading drain parser from {}", path.display());

		let mut file = File::open(path).unwrap();

		self.load_reader(&mut file);

		println!("Loaded drain parser in {:?}", timer.elapsed());
	}

	pub fn load_reader(&mut self, reader: &mut impl Read) {
		let magic = reader.read_u32::<LittleEndian>().unwrap();

		if magic != DRAIN_MAGIC {
			panic!("Invalid magic");
		}
		
		let version = reader.read_u32::<LittleEndian>().unwrap();

		if version != CURR_DRAIN_MODEL_VERSION {
			panic!("Unknow version");
		}

		self.new_id = reader.read_u32::<LittleEndian>().unwrap() as usize;

		loop {
			let flag = reader.read_u16::<LittleEndian>().unwrap();

			if flag == END_CLUSTER_GROUP {
				break;
			}

			if flag != BEGIN_CLUSTER_GROUP {
				panic!("expected BEGIN_CLUSTER_GROUP");
			}

			let len = reader.read_u32::<LittleEndian>().unwrap();

			let mut map = HashMap::new();

			loop {
				let flag = reader.read_u16::<LittleEndian>().unwrap();

				if flag == END_EVENT_GROUP {
					break;
				}

				if flag != BEGIN_EVENT_GROUP {
					panic!("expected BEGIN_EVENT_GROUP");
				}

				let event_len = reader.read_u32::<LittleEndian>().unwrap();
				let mut event = vec![0; event_len as usize];
				reader.read_exact(&mut event).unwrap();

				let event = String::from_utf8(event).unwrap();

				let mut cluster = LogCluster {
					len: len as usize,
					templates: Vec::new(),
				};

				loop {
					let flag = reader.read_u16::<LittleEndian>().unwrap();

					if flag == END_TEMPLATE {
						break;
					}

					if flag != BEGIN_TEMPLATE {
						panic!("expected BEGIN_TEMPLATE");
					}

					let template_id = reader.read_u32::<LittleEndian>().unwrap(); 
					let count = reader.read_u32::<LittleEndian>().unwrap();

					let mut template = Template {
						id: template_id as usize,
						count: count as usize,
						tokens: Vec::new(),
					};

					let token_count = reader.read_u32::<LittleEndian>().unwrap();

					for _ in 0..token_count {
						let len = reader.read_u32::<LittleEndian>().unwrap();
						let mut token = vec![0; len as usize];
						reader.read_exact(&mut token).unwrap();
						let token = String::from_utf8(token).unwrap();

						template.tokens.push(token);
					}

					cluster.templates.push(template);
				}

				map.insert(event, cluster);
			}

			self.parsers.insert(len as usize, map);
		}
	}
}


#[derive(Debug, PartialEq)]
pub struct Template {
    pub id: usize,
    pub tokens: Tokens,
    pub count: usize,
}

#[derive(Debug, PartialEq)]
pub struct LogCluster {
    pub len: usize,
    pub templates: Vec<Template>
}

impl LogCluster {
    pub fn new(len: usize) -> Self {
        LogCluster {
            len,
            templates: Vec::new()
        }
    }

    pub fn process(&mut self, tokens: Tokens, new_id: usize) -> DrainParseOutput {
        let l = tokens.len();

        let mut largest_score = None;

        for (index, template) in self.templates.iter().enumerate() {
            let score = simseq(&tokens, &template.tokens);

            match largest_score {
                None => largest_score = Some((index, score)),
                Some((_, s)) => {
                    if score > s {
                        largest_score = Some((index, score));
                    }
                }
            }
        }

        let template_index = match largest_score {
            None => {
                let template = Template {
                    id: new_id,
                    tokens: tokens.clone(),
                    count: 1
                };
                self.templates.push(template);

                self.templates.len() - 1
            },
            Some((index, score)) => {
                let threshold = score as f32 / self.len as f32;

                if threshold >= 0.5 {
                    let mut template = &mut self.templates[index];
                    template.count += 1;

                    for i in 0..l {
                        if template.tokens[i] != tokens[i] {
                            template.tokens[i] = "*".to_string();
                        }
                    }

                    index
                } else {
                    let template = Template {
                        id: new_id,
                        tokens: tokens.clone(),
                        count: 1
                    };

                    self.templates.push(template);

                    self.templates.len() - 1
                }
            }
        };

        let template = &self.templates[template_index];

        let mut parameters = Vec::new();

        for i in 0..l {
            if template.tokens[i] == "*" {
                parameters.push(tokens[i].clone());
            }
        }

        DrainParseOutput {
            template: template.id,
            parameters: parameters,
            tokens: template.tokens.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

	#[test]
	fn test_drain_parser_saving() {
		let mut drain = super::DrainParser::new();

		drain.parse("A A A");
		drain.parse("A B A");
		drain.parse("q w e r t y");

		let mut c = Cursor::new(vec![]);

		drain.save_writer(&mut c);

		let mut drain2 = super::DrainParser::new();

		c.set_position(0);

		drain2.load_reader(&mut c);

		assert_eq!(drain, drain2);
	}
}