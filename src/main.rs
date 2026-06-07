use std::path::{Path, PathBuf};
use std::process;
use std::{env, fs};

#[derive(Debug)]
struct Source {
    name: String,
    dir: String,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Include {
    name: String,
    dir: String,
}

#[derive(Debug)]
struct Database {
    project_root_dir: String,
    source_type: String,
    args: Vec<String>,
    sources: Vec<Source>,
    includes: Vec<Include>,
}

impl Database {
    fn new() -> Database {
        Database {
            project_root_dir: String::new(),
            source_type: String::new(),
            args: vec![],
            sources: vec![],
            includes: vec![],
        }
    }
    fn set_root(&mut self, dir: String) {
        self.project_root_dir = dir;
    }
    fn get_root(&mut self) -> &str {
        &self.project_root_dir
    }
    fn set_type(&mut self, r#type: String) {
        self.source_type = r#type;
    }
    fn get_type(&mut self) -> &str {
        &self.source_type
    }
    fn push_arg(&mut self, arg: String) {
        self.args.push(arg);
    }
    fn get_args(&mut self) -> &Vec<String> {
        &self.args
    }
    fn push_source(&mut self, source: Source) {
        self.sources.push(source);
    }
    fn get_sources(&mut self) -> &Vec<Source> {
        &self.sources
    }
    fn push_include(&mut self, include: Include) {
        self.includes.push(include);
    }
    fn get_includes(&mut self) -> &Vec<Include> {
        &self.includes
    }
    fn sort_name(&mut self) {
        let _ = &self
            .sources
            .sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.dir.cmp(&b.dir)));
        let _ = &self
            .includes
            .sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.dir.cmp(&b.dir)));
    }
    fn sort_incs_dir(&mut self) {
        let _ = &self.includes.sort_by(|a, b| a.dir.cmp(&b.dir));
    }
}

fn recursive_search(dir: PathBuf, base: &mut Database) {
    let dir_name = dir.to_str().expect("fail to parse directory").to_string();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if let Some(name) = name.to_str() {
                if path.is_dir() {
                    recursive_search(path, base);
                } else if name.ends_with(".h") {
                    base.push_include(Include {
                        dir: dir_name.clone(),
                        name: name.into(),
                    });
                } else if name.ends_with(match base.get_type() {
                    "c" => ".c",
                    "cpp" => ".cpp",
                    _ => "can't be this",
                }) {
                    base.push_source(Source {
                        dir: dir_name.clone(),
                        name: name.into(),
                    });
                }
            }
        }
    }
}

fn main() {
    let args = env::args();
    let root = env::current_dir().expect("fail to parse directory");

    let mut data = Database::new();
    /* root */
    data.set_root(
        root.clone()
            .into_os_string()
            .into_string()
            .expect("fail to parse os string"),
    );
    /* args_1 must be "c" or "cpp", then if there are additional args, must use -a */
    let mut args = args.skip(1).collect::<Vec<String>>();
    if args.is_empty() {
        eprintln!("need to set the type of sources");
        process::exit(-1);
    }
    let r#type = args.remove(0);
    match r#type.as_str() {
        "c" | "cpp" => data.set_type(r#type),
        _ => {
            eprintln!("wrong source type, must be \"c\" or \"cpp\"");
            process::exit(-1);
        }
    }
    for (i, a) in args.iter().enumerate() {
        if i == 0 {
            if a != "-a" {
                eprintln!("need to use \"-a\" to set the arguments");
                process::exit(-1);
            } else {
                continue;
            }
        }
        data.push_arg(a.into());
    }

    /* search all the directories */
    recursive_search(root, &mut data);

    if data.get_sources().is_empty() {
        println!("no sources");
        process::exit(0);
    }

    /* get the includes' dirs */
    data.sort_incs_dir();
    let mut incs_dir = String::new();
    let mut pre = "";

    for i in data.get_includes() {
        if i.dir != pre {
            incs_dir += format!("      \"-I{}\",\r\n", i.dir).as_str();
        }
        pre = i.dir.as_str();
    }

    /* root_dir */
    let root = String::from(format!("    \"directory\": \"{}\",\r\n", data.get_root()));

    /* args */
    let mut args = String::from("    \"arguments\": [\r\n");
    for a in data.get_args() {
        args += format!("      \"{}\",\r\n", a).as_str();
    }
    args += &incs_dir;
    args.pop();
    args.pop();
    args.pop();
    args += "\r\n    ],\r\n";

    /* all the datas */
    let separator;
    #[cfg(unix)]
    {
        separator = "/";
    }
    #[cfg(windows)]
    {
        separator = "\\";
    }

    /* begin */
    let mut datas = String::from("[\r\n");

    for s in data.get_sources() {
        datas += "  {\r\n";
        datas += root.as_str();
        datas += args.as_str();
        datas += format!("    \"file\": \"{}{}{}\"\r\n", s.dir, separator, s.name).as_str();
        datas += "  },\r\n";
    }
    datas.pop();
    datas.pop();
    datas.pop();
    datas += "\r\n";

    /* end */
    datas += "]\r\n";

    let mut file = data.get_root().to_string();
    file += separator;
    file += "compile_commands.json";

    let path = Path::new(&file);
    if let Err(err) = fs::write(path, datas) {
        eprintln!("{err}");
    }

    /* print the duplicate datas */
    data.sort_name();
    let mut flag_start = true;
    let mut flag_dup = false;
    let mut pre_name = "";
    let mut pre_dir = "";

    let incs = data.get_includes();
    for i in incs {
        if pre_name == i.name {
            if flag_start {
                flag_start = false;
                println!("duplicate files:");
            }
            println!("header: {}", pre_dir.to_string() + separator + pre_name);
            flag_dup = true;
        } else if flag_dup {
            flag_dup = false;
            println!("header: {}", pre_dir.to_string() + separator + pre_name);
        }
        pre_dir = &i.dir;
        pre_name = &i.name;
    }
    if flag_dup {
        println!("header: {}", pre_dir.to_string() + separator + pre_name);
    }

    pre_name = "";
    pre_dir = "";
    flag_dup = false;
    let srcs = data.get_sources();
    for s in srcs {
        if pre_name == s.name {
            if flag_start {
                flag_start = false;
                println!("duplicate files:");
            }
            println!("src: {}", pre_dir.to_string() + separator + pre_name);
            flag_dup = true;
        } else if flag_dup {
            flag_dup = false;
            println!("src: {}", pre_dir.to_string() + separator + pre_name);
        }
        pre_dir = &s.dir;
        pre_name = &s.name;
    }
    if flag_dup {
        println!("src: {}", pre_dir.to_string() + separator + pre_name);
    }
}
