#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(warnings)] // Disables all warnings for the entire crate
#![allow(unused)] // Disables all 'unused' related warnings (variables, code, imports)
mod action;
mod app;

use action::{Modify,Operation,InsertMode};
use app::FileRenamerApp;
use iced::{Size, Settings};
use regex::Regex;

fn main() -> iced::Result {
    
    iced::application(FileRenamerApp::default, FileRenamerApp::update, FileRenamerApp::view)
    .subscription(FileRenamerApp::subscription)
    .title("File Renamer")
    .window_size(Size::new(1600.0,900.0))
        /* .window(iced::window::Settings {
            size: Size::new(1280.0, 1280.0),
            resizable: true,
            ..Default::default()
        })*/
        .centered()
        .run()
     //iced::run(FileRenamerApp::update, FileRenamerApp::view)
    
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::action::Action;

    use super::*;
    #[test]
    fn test_affix() {
    let mut modify = Modify::new_affix();
    let pattern = String::from("modified");
    modify.prefix_mode(&pattern);
    let mut test_path = PathBuf::from("/A/B/FooBar.txt");

    modify.action(&mut test_path);
    assert_eq!(test_path,PathBuf::from("/A/B/modifiedFooBar.txt"));

    modify.suffix_mode(&pattern);

    modify.action(&mut test_path);
    assert_eq!(test_path,PathBuf::from("/A/B/modifiedFooBarmodified.txt"));

    }

      #[test]
    fn test_find_and_replace() {
        let mut modify = Modify::new_op();
        modify
        .set_pattern("Foo")
        .find_and_replace_op(&"Modified".to_string());
        let mut test_path = PathBuf::from("/A/B/FooBar.txt");

        modify.action(&mut test_path);
        assert_eq!(test_path,PathBuf::from("/A/B/ModifiedBar.txt"));

        modify
        .set_pattern("Bar")
        .find_and_replace_op(&"".to_string());
        modify.action(&mut test_path);
        assert_eq!(test_path,PathBuf::from("/A/B/Modified.txt"));
        
    }

    #[test]
    fn test_regex_replace() {
    let mut modify = Modify::new_op();

    modify.set_pattern(r"^Foo")
    .regex_op(&"Modified".to_string());

    let mut test_path = PathBuf::from("/A/B/FooBar.txt");
    modify.action(&mut test_path);

    assert_eq!(test_path,PathBuf::from("/A/B/ModifiedBar.txt"));
    }

    #[test]
    fn test_regex_fail(){
    // This will panic because the `(` is not closed.
    //let re = Regex::new(r"(").unwrap();
    let mut modify = Modify::new_op();

    modify.set_pattern(r"(")
    .regex_op(&"test".to_string());

    let mut test_path = PathBuf::from("/A/B/FooBar.txt");
    modify.action(&mut test_path);

    assert_eq!(test_path,PathBuf::from("/A/B/FooBar.txt"));
    }


}