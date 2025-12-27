use iced::event::{self, Event};
use iced::keyboard::{self, key};
use iced::Length::FillPortion;
use iced::font;
use iced::wgpu::naga::back::hlsl::Options;
use iced::widget::{operation};
use iced::widget::{container, rule, scrollable, space};
use iced::widget::{button, column, pick_list, row, table, text, text_input, toggler, tooltip};
use iced::{Center,Element,Fill,Font, Padding, Task, Theme, Renderer,Subscription};
use std::path::PathBuf;
use tokio::fs;
use tokio::task::JoinSet;
use rfd::{AsyncFileDialog, MessageDialogResult};

use crate::action::{Action,Modify, Replace, StateValue};



#[derive(Debug, Clone)]
pub enum Message {
    AddAction,
    UpdateAction(usize,ActionOptions),
    RemoveAction(usize),
    ApplyChange,
    ChangeApplied(Vec<RenameError>),
    PatternChange(usize,String),
    TextChange(usize,String),
    OpenFolderPicker,
    FolderSelected(Option<PathBuf>),
    OpenMultiPicker,
    FilesSelected(Option<Vec<PathBuf>>),
    PopulateTable(Vec<PathBuf>),
    Preview,
    TogglePreview(bool),
    Event(Event),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ActionOptions {
        #[default]
        MatchAndReplace,
        RegexReplace,
        Prefix,
        Suffix,
        //UpperCase,
        //LowerCase,
}

pub struct TextState {
        pattern: String,
        value: String,
        action_option: Option<ActionOptions>,
}

impl Default for TextState {
    fn default() -> Self {
        Self {
            pattern: String::new(),
            value: String::new(),
            action_option: Some(ActionOptions::default()),
        }
    }
}

//#[derive(Default)]
pub struct FileRenamerApp {
        //current_file_names: Vec<String>,
        //modified_file_names: Vec<String>,
        //current , modified
        file_names: Vec<(PathBuf,PathBuf)>,
        texts_state: Vec<TextState>,
        live_preview: bool,
        actions: Vec<Box<dyn Action>>,
}

impl Default for FileRenamerApp {
    fn default() -> Self {
        Self {
            file_names: Vec::new(),
            texts_state: Vec::new(),
            live_preview: true,
            actions: Vec::new(),
        }
    }
}

impl FileRenamerApp {
        pub fn subscription(&self) -> Subscription<Message> {
                event::listen().map(Message::Event)
        }

        pub fn view(&self) -> Element<'_, Message> {
                let mut content = column![
                row![button("Choose Files").on_press(Message::OpenMultiPicker),
                button("Choose Folder").on_press(Message::OpenFolderPicker),
                button("Add").on_press(Message::AddAction)].spacing(10)
                ]
                .spacing(20)
                .padding(20).align_x(Center);

                let mut scrollable_content =column![].spacing(30).padding(20).align_x(Center);
                for (index, _ ) in self.texts_state.iter().enumerate() {
                        scrollable_content = scrollable_content.push(self.action_input_ui(index));
                        scrollable_content = scrollable_content.push(rule::horizontal(1.0));
                }
                content = content.push(scrollable(scrollable_content).height(Fill));
                content = content.push(rule::horizontal(1.0));
                //scrollable(content).width(FillPortion(1)).into()
                content = content.push(row![space::horizontal(),
                tooltip(
                        toggler(self.live_preview).label("Live Preview").on_toggle(Message::TogglePreview),
                        "Turn off live preview if its laggy for large amount of files",
                        tooltip::Position::Top
                ),
                if self.live_preview {button(text("Preview").align_x(Center))
                        } else {
                        button(text("Preview").align_x(Center))
                        .on_press(Message::Preview)},
                button(text("Apply").align_x(Center))
                .on_press(Message::ApplyChange)
                .width(80)].spacing(10).align_y(Center));

                row! [
                content.width(FillPortion(8)),
                scrollable(self.display_ui()).width(FillPortion(13))]
                .into()
                //scrollable(self.display_ui()).width(Fill).into()

                //content.into()
    } 

        pub fn update(&mut self, message: Message) -> Task<Message>{
                match message {
                        Message::AddAction => {
                                self.texts_state.push(TextState::default());
                                self.actions.push(Box::new(Modify::new_op()));
                                Task::none()
                        },
                        Message::UpdateAction(i,selected_action) =>{
                                self.texts_state[i].action_option = Some(selected_action);
                                let new_action: Box<dyn Action>;

                                match self.texts_state[i].action_option {
                                        Some(ActionOptions::MatchAndReplace) => {
                                                let mut action = Modify::new_op();
                                                action
                                                .set_pattern(&self.texts_state[i].pattern)
                                                .find_and_replace_op(&self.texts_state[i].value);
                                                new_action = Box::new(action);
                                        },
                                        Some(ActionOptions::RegexReplace) => {
                                                let mut action = Modify::new_op();
                                                action
                                                .set_pattern(&self.texts_state[i].pattern)
                                                .regex_op(&self.texts_state[i].value);
                                                new_action = Box::new(action);
                                        },
                                        Some(ActionOptions::Prefix) => {
                                                let mut action = Modify::new_affix();
                                                action.prefix_mode(&self.texts_state[i].value);
                                                new_action = Box::new(action);
                                        },
                                        Some(ActionOptions::Suffix) => {
                                                let mut action = Modify::new_affix();
                                                action.suffix_mode(&self.texts_state[i].value);
                                                new_action = Box::new(action);
                                        },
                                        _ =>{ panic!("Should never have None ActionOptions");}
                                }
                                self.actions[i] = new_action;
                                if self.live_preview {self.preview_new_filename()};
                                Task::none()
                        },
                        Message::RemoveAction(i) => {
                                self.texts_state.remove(i);
                                self.actions.remove(i);
                                if self.live_preview{self.preview_new_filename()};
                                Task::none()
                        },
                        Message::ApplyChange => {
                                self.preview_new_filename();
                                Task::perform(rename_files(self.file_names.clone()), Message::ChangeApplied)
                        },
                        Message::ChangeApplied(rename_error) => {
                                for (old, new) in self.file_names.iter_mut() {
                                        *old = new.clone();
                                }
                                Task::future(ok_dialog()).discard()
                        }
                        Message::PatternChange(i,pattern) => {
                                self.texts_state[i].pattern = pattern.clone();
                                let value = self.texts_state[i].value.clone();

                                match self.texts_state[i].action_option {
                                        Some(ActionOptions::MatchAndReplace) | Some(ActionOptions::RegexReplace) => {
                                                self.actions[i].update_values(
                                                        StateValue::ReplaceValue(pattern,value));
                                                },
                                                _ =>{}
                                        }
                                        if self.live_preview{self.preview_new_filename()};

                                Task::none()
                        },
                        Message::TextChange(i,value) => {
                                self.texts_state[i].value = self.filter_invalid(value);
                                let filtered_value = self.texts_state[i].value.clone();
                                let pattern = self.texts_state[i].pattern.clone();

                                match self.texts_state[i].action_option {
                                        Some(ActionOptions::MatchAndReplace) | Some(ActionOptions::RegexReplace) => {
                                                self.actions[i].update_values(
                                                        StateValue::ReplaceValue(pattern,filtered_value));
                                                },
                                        Some(ActionOptions::Prefix) | Some(ActionOptions::Suffix) => {
                                                self.actions[i].update_values(StateValue::AffixValue(filtered_value));
                                                },
                                                _ =>{}
                                        }
                                        if self.live_preview{self.preview_new_filename()};

                                Task::none()
                        },
                        Message::OpenFolderPicker => {
                                Task::perform(pick_folder(), Message::FolderSelected)
                        },
                        Message::OpenMultiPicker => {
                                Task::perform(pick_files(), Message::FilesSelected)
                        },
                        Message::FilesSelected(Some(files)) => {
                                self.file_names = files.into_iter().map(|path_buf| {

                                        (path_buf.clone(),path_buf)
                                }).collect();

                                Task::none()
                        },
                        Message::FolderSelected(path) => {
                                if let Some(dir_path) = path {
                                        Task::perform(read_files_from_folder(dir_path), Message::PopulateTable)
                                } else {
                                        Task::none()
                                }
                        },
                        Message::PopulateTable(file_names)=> {                      
                                self.file_names = file_names.into_iter().map(|path_buf| {                                       
                                        (path_buf.clone(), path_buf)
                                }).collect();
                                
                                Task::none()
                        },
                        Message::Preview => {
                                self.preview_new_filename(); Task::none()
                        },
                        Message::TogglePreview(is_on) => {
                                self.live_preview = is_on; Task::none()
                        },
                        Message::Event(event) => match event {
                                Event::Keyboard(keyboard::Event::KeyPressed {
                                        key: keyboard::Key::Named(key::Named::Tab),
                                        modifiers,
                                        ..
                                }) => {
                                        if modifiers.shift() {
                                                operation::focus_previous()
                                        } else {
                                                operation::focus_next()
                                        }}
                                        _ => Task::none()
                        },
                        _ => Task::none()
                }
        }

        fn preview_new_filename(&mut self) {
                for (old_path, new_path) in self.file_names.iter_mut() {
                        *new_path = old_path.clone();
                        for renamer in &mut self.actions {
                                *new_path = renamer.action(new_path);
                                }

        }
}
        fn filter_invalid(&self, input: String) -> String{
                let invalid = ['<','>',':','"','/','\\','|','?','*','.'];
                let filter_str :String = input
                .chars()
                .filter(|c| !invalid.contains(c))
                .collect();
                filter_str
        }
        fn test_button<'a>(&self) -> Element<'a,Message>{
                button("Test").on_press(Message::AddAction).into()}

        fn action_input_ui<'a>(&self,index: usize) -> Element<'a,Message> {
                let mut content= row![];

                let text_state = &self.texts_state[index];
                
                let (pattern_label,value_label) = match &text_state.action_option {
                        Some(ActionOptions::MatchAndReplace) =>{
                                ("Match".to_string(),"Replace".to_string())
                        },
                        Some(ActionOptions::RegexReplace) =>{
                                ("Regex".to_string(),"Replace".to_string())
                        },
                        Some(ActionOptions::Prefix) =>{
                                ("".to_string(),"Prefix".to_string())
                        },
                        Some(ActionOptions::Suffix) =>{("".to_string(),"Suffix".to_string())},
                        _ =>{("_".to_string(),"_".to_string())},
                };

                
                
                if let Some(ActionOptions::MatchAndReplace) | Some(ActionOptions::RegexReplace) = &text_state.action_option {
                        content = content.push(row! [
                                text!("{}:",pattern_label).width(55),//.align_x(Center).align_y(Center),
                                text_input("", &text_state.pattern).on_input(move |s| Message::PatternChange(index, s)),
                        ]
                        .spacing(10)
                        .align_y(Center));
                }

           content.push(
                row![
                     //text!("{}:",pattern_label).width(70).align_y(Center),
                     //text_input("", &text_state.pattern).on_input(move |s| Message::PatternChange(index, s)),
                     text!("{}:",value_label).width(55).align_y(Center),
                     text_input("", &text_state.value).on_input(move |s| Message::TextChange(index, s)),
                     self.action_picker_ui(index),
                     button("Delete").on_press(Message::RemoveAction(index)).style(button::danger),

                ].spacing(10)
                .align_y(Center)
        ).spacing(10)
        .wrap()
        .into()

        }

        fn action_picker_ui<'a>(&self, index: usize) -> Element<'a,Message> {
                let pick_options = [
                        ActionOptions::MatchAndReplace,
                        ActionOptions::RegexReplace,
                        ActionOptions::Prefix,
                        ActionOptions::Suffix,
                ];
                pick_list(pick_options,
                        self.texts_state[index].action_option,
                        move |selected| Message::UpdateAction(index, selected),

                ).into()
        }

        fn display_ui<'a>(&self) -> Element<'_,Message> {
                //let row_data: &Vec<(&String,&String)>= &self.current_file_names.iter().zip(&self.modified_file_names).collect();
                
                let table_content = {
                let bold = |header| /*-> iced::widget::Text<'a, Theme>*/ {
                text(header).font(Font {
                    weight: font::Weight::Bold,
                    ..Font::DEFAULT
                }).width(Fill)};
                
                //let arr:Vec<i32> = (1..100).collect();
                //let st:Vec<String> = (1..100).map(|i| format!("Hi {}", i)).collect();
                //let rows:Vec<(i32,String)> = arr.into_iter().zip(st).collect();

                let columns = [
                        table::column(   bold("Original File Name"), 
                        |row: &(PathBuf, PathBuf)| {
                                if let Some(file_name) = &row.0.file_name() {
                                        let filename_str = file_name.to_string_lossy();
                                        text(filename_str)
                                } else {text("Unable to load file")}
                          })
                        .width(FillPortion(1)),
                        table::column(bold("Modified File Name"), 
                        |row: &(PathBuf, PathBuf)| {
                                if let Some(file_name) = &row.1.file_name() {
                                        let filename_str = file_name.to_string_lossy();
                                        text(filename_str)
                                } else {text("Unable to load file")}
                          })
                        .width(FillPortion(1))
                        ];
                table(columns, &self.file_names).padding(10)
                };

                column![row![rule::vertical(1.0),table_content],rule::horizontal(1.0)].into()
 

                
        }
}
async fn pick_folder() -> Option<PathBuf> {
        let handle = AsyncFileDialog::new()
                .set_title("Choose a Directory")
                .pick_folder() 
                .await;
        // Map the FileHandle to a PathBuf if a folder was selected
        handle.map(|h| h.path().to_path_buf())              
}

async fn pick_files() -> Option<Vec<PathBuf>> {
    let handles = AsyncFileDialog::new()
        .set_title("Select Multiple Files")
        // You can add filters to restrict file types
        //.add_filter("Documents", &["txt", "pdf", "doc"])
        .pick_files() // Note the 's' at the end
        .await?; // Returns early if user cancels

    // Convert Vec<FileHandle> to Vec<PathBuf>
    let paths = handles
        .into_iter()
        .map(|h| h.path().to_path_buf())
        .collect();

    Some(paths)
}

async fn read_files_from_folder(path: PathBuf) -> Vec<PathBuf> {
        let mut file_names = Vec::new();
    
        // Handle the Result here. If it fails, return an empty Vec.
        if let Ok(mut read_dir) = fs::read_dir(&path).await {
                while let Ok(Some(entry)) = read_dir.next_entry().await {
                if entry.path().is_file() {
                        file_names.push(entry.path());//.file_name().to_string_lossy().into_owned());
                        }
                }
        }
        file_names
}

async fn rename_files(files: Vec<(PathBuf, PathBuf)>) -> Vec<RenameError> {
        let mut set = JoinSet::new();
        let mut failures = Vec::new();

    for (old_path, new_path) in &files {
        let old = old_path.clone();
        let new = new_path.clone();

        set.spawn(async move {
            // Check if the file still existing in case its moved before rename
            if !old.exists() {
                return Err(RenameError {
                    old_path: old,
                    error: "Source no longer exists".into(),
                });
            }
            /*// Check if the destination parent directory still exists
            if let Some(parent) = new.parent() {
                if !parent.exists() {
                    return Err(RenameError {
                        old_path: old,
                        error: "Destination folder is missing".into(),
                    });
                }
            }*/

            fs::rename(&old, &new).await.map_err(|e| RenameError {
                old_path: old,
                error: e.to_string(),
            })
        });
    }

    //let mut final_result = Ok(());

    // Wait for all tasks to complete
    while let Some(res) = set.join_next().await {
        match res {
            Ok(Err(rename_err)) => failures.push(rename_err),
            Err(join_err) => {
                // This happens if a task panics
                eprintln!("Worker thread failed: {:?}", join_err);
            },
            _ => {}
        }
    }

    failures
}

async fn confirmation_dialog() -> MessageDialogResult{
            rfd::AsyncMessageDialog::new()
                .set_title("Confirm Rename")
                .set_description("Are you sure you want to rename these files?")
                .set_buttons(rfd::MessageButtons::YesNo)
                .show()
                .await
        }
async fn ok_dialog() {
        let dialog = rfd::AsyncMessageDialog::new()
        .set_title("File Renamer")
        .set_description("Files has been renamed.")
        .set_buttons(rfd::MessageButtons::Ok)
        .show()
        .await;
}

#[derive(Debug, Clone)]//currently not used
pub struct RenameError {
    pub old_path: PathBuf,
    pub error: String,
}
impl std::fmt::Display for ActionOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MatchAndReplace => "Match and Replace",
            Self::RegexReplace => "Regex Replace",
            Self::Prefix => "Add Prefix",
            Self::Suffix => "Add Suffix",
            //Self::UpperCase => "UPPER CASE",
            //Self::LowerCase => "lower case",
        })
    }
}