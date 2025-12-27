
use std::{default, marker::PhantomData, path::PathBuf,fs};
use regex::Regex;


#[derive(Debug)]
pub struct Affix {
        mode: InsertMode,
        str_to_add: String,
}

#[derive(Debug)]
pub struct Replace {
        operation: Operation,
        pattern: String,
        replace_str: String,
}

/*#[derive(Debug)]
pub struct LetterCase {

}*/

#[derive(Debug)]
pub enum Operation {
        ReplaceString,
        ReplaceRegex,
        Remove,

}
#[derive(Debug)]
pub enum InsertMode {
        Prefix,
        //Infix,
        Suffix,
}

pub enum StateValue {
    AffixValue(String),
    ReplaceValue(String, String),
}
pub trait Action {
        fn action(&self, file_name: &PathBuf) -> PathBuf;

        fn update_values(&mut self,data: StateValue);

        fn rename_file(&self, old_name: &PathBuf,new_name: &PathBuf) {//blocking
                if !old_name.exists()
                        {return;}
                std::fs::rename(old_name, new_name);
        }
}

#[derive(Debug)]
pub struct Modify<Mode> {
        //pattern: String,
        //operation: Operation<'a>,
        //mode: InsertMode<'a>,
        state: Mode,
}

impl Modify<Replace> {
        pub fn new_op() -> Modify<Replace> {
                Self {
                state:Replace { operation: Operation::ReplaceString,
                        pattern: String::new(),
                        replace_str:String::new() }
                }
        }
        pub fn set_pattern(&mut self,pattern: &str) -> &mut Self {
                self.state.pattern = pattern.to_owned();
                self
        }

        pub fn find_and_replace_op(&mut self,string_to_replace: &String) {
                self.state.replace_str = string_to_replace.to_owned();
                self.state.operation = Operation::ReplaceString;
        }

        pub fn regex_op(&mut self,string_to_replace: &String) {
                self.state.replace_str = string_to_replace.to_owned();
                self.state.operation = Operation::ReplaceRegex;
        }

        pub fn remove_op(&mut self) {
                self.state.operation = Operation::Remove;
        }

        fn perform_operation(&self,path: &mut PathBuf) {
                let extension = path.extension().map(|ext| ext.to_os_string());
                let mut value = String::new();
                if let Some(file_name) = path.file_stem() {
                        value = file_name.to_string_lossy().into_owned();
                } else{ return; /*Do not modify if its none*/ };

                match self.state.operation {
                    Operation::ReplaceString => { value = value.replacen(&self.state.pattern, &self.state.replace_str,1);},
                    Operation::ReplaceRegex => {
                        let re = Regex::new(&self.state.pattern);
                        if let Some(re) = re.ok() {
                                 let result = re.replace(&value, &self.state.replace_str);
                                 value = result.into_owned();
                        }//do nothing if regex fail
                        
                    },
                    Operation::Remove => {value = value.replace(&self.state.pattern,"");},
                    _ => println!("String modification failed"),
                }
                path.set_file_name(value);
                if let Some(ext) = extension {
                        path.set_extension(ext);
                }

        }
}

impl Action for Modify<Replace> { 
        fn action(&self,file_name: &PathBuf) -> PathBuf {
                let mut new_name = file_name.to_owned();
                self.perform_operation(&mut new_name);
                new_name
        }

        fn update_values(&mut self,data: StateValue) {

                if let StateValue::ReplaceValue(pattern,value) = data {
                        self.state.pattern = pattern;
                        self.state.replace_str = value;

                };
            
        }
}

impl Modify<Affix> {
        pub fn new_affix() -> Modify<Affix> {
                Self {
                state: Affix { mode: InsertMode::Suffix,
                        str_to_add: String::new() }
                }
        }

        pub fn prefix_mode(&mut self,value: &str) {
                self.state.str_to_add = value.to_owned();
                self.state.mode = InsertMode::Prefix;
        }

        pub fn suffix_mode(&mut self,value: &str) {
                self.state.str_to_add = value.to_owned();
                self.state.mode = InsertMode::Suffix;
        }

        fn add_affix(&self,path: &mut PathBuf) {
                let extension = path.extension().map(|ext| ext.to_os_string());
                let mut value = String::new();
                if let Some(file_name) = path.file_stem() {
                        value = file_name.to_string_lossy().into_owned();
                } else{ return;/*Do not modify if its none*/ };

                match self.state.mode {
                        InsertMode::Prefix => { value.insert_str(0, &self.state.str_to_add)},
                        InsertMode::Suffix => { value.push_str(&self.state.str_to_add);},
                        _ => {println!("Error adding affix");},
                }
                path.set_file_name(value);
                if let Some(ext) = extension {
                        path.set_extension(ext);
                }
        }
        
}

impl Action for Modify<Affix> {
        
         fn action(&self,file_name: &PathBuf ) ->PathBuf {
                let mut new_name = file_name.to_owned();
                self.add_affix(&mut new_name);
                new_name
        }

        fn update_values(&mut self,data: StateValue) {
                if let StateValue::AffixValue(value) = data {
                        self.state.str_to_add = value;
                };   
        }
}

/*impl Modify<LetterCase> {
    
}*/
