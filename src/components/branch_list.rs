use std::{future::Future, pin::Pin, process::Output};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::Text,
  widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use tracing::error;

use crate::{
  action::Action,
  components::{
    branch_list::{branch_input::BranchInput, branch_item::BranchItem, instruction_footer::InstructionFooter},
    Component,
  },
  error::Error,
  git::git_wrapper::{
    git_checkout_branch_from_name, git_create_branch, git_delete_branch, git_local_branches, GitBranch,
  },
  tui::Frame,
};

mod branch_input;
mod branch_item;
mod instruction_footer;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
  Selection,
  Input,
}

pub struct BranchList {
  mode: Mode,
  error: Option<String>,
  // List state
  branches: Vec<BranchItem>,
  list_state: ListState,
  selected_index: usize,
  // Components
  branch_input: BranchInput,
  instruction_footer: InstructionFooter,
}

impl Default for BranchList {
  fn default() -> Self {
    // Assume branch names are all valid as they come from git
    BranchList {
      mode: Mode::Selection,
      error: None,
      branches: Vec::new(),
      list_state: ListState::default(),
      selected_index: 0,
      branch_input: BranchInput::new(),
      instruction_footer: InstructionFooter::default(),
    }
  }
}

impl BranchList {
  pub async fn load(&mut self) {
    let branches: Vec<BranchItem> =
      git_local_branches().await.unwrap().iter().map(|branch| BranchItem::new(branch.clone(), true)).collect();
    self.branches = branches;
  }

  pub fn clear_error(&mut self) {
    self.error = None;
  }

  pub fn select_previous(&mut self) {
    if self.selected_index == 0 {
      self.selected_index = self.branches.len() - 1;
      return;
    }
    if self.selected_index >= self.branches.len() {
      self.selected_index = self.branches.len() - 1;
      return;
    }
    self.selected_index -= 1;
  }

  pub fn select_next(&mut self) {
    if self.selected_index == self.branches.len() - 1 {
      self.selected_index = 0;
      return;
    }
    if self.selected_index >= self.branches.len() {
      self.selected_index = 0;
      return;
    }
    self.selected_index += 1;
  }

  fn get_selected_branch(&self) -> Option<&BranchItem> {
    self.branches.get(self.selected_index)
  }

  async fn checkout_selected(&mut self) -> Result<(), Error> {
    let maybe_selected = self.get_selected_branch();
    if maybe_selected.is_none() {
      return Ok(());
    }
    let name_to_checkout = maybe_selected.unwrap().branch.name.clone();
    git_checkout_branch_from_name(&name_to_checkout).await?;
    for existing_branch in self.branches.iter_mut() {
      existing_branch.branch.is_head = existing_branch.branch.name == name_to_checkout;
    }
    Ok(())
  }

  pub fn stage_selected_for_deletion(&mut self, stage: bool) {
    let maybe_selected = self.branches.get_mut(self.selected_index);
    if maybe_selected.is_none() {
      return;
    }
    let selected = maybe_selected.unwrap();
    if selected.branch.is_head {
      return;
    }
    selected.stage_for_deletion(stage);
  }

  pub async fn deleted_selected(&mut self) -> Result<(), Error> {
    let selected = self.branches.get(self.selected_index);
    if selected.is_none() {
      return Ok(());
    }
    let delete_result = git_delete_branch(&selected.unwrap().branch).await;
    if delete_result.is_err() {
      return Ok(());
    }
    self.branches.remove(self.selected_index);
    if self.selected_index >= self.branches.len() {
      self.selected_index -= 1;
    }
    Ok(())
  }

  pub async fn delete_staged_branches(&mut self) -> Result<(), Error> {
    let mut indexes_to_delete: Vec<usize> = Vec::new();

    for branch_index in 0..self.branches.len() {
      let branch_item = &self.branches[branch_index];
      if !branch_item.staged_for_deletion {
        continue;
      }
      let del_result = git_delete_branch(&branch_item.branch).await;
      if del_result.is_ok() {
        indexes_to_delete.push(branch_index);
      } else {
        // TODO communicate deletion error
      }
    }

    // Sort and reverse, so we remove branches starting from the end,
    // which means we don't need to worry about changing array positions.
    indexes_to_delete.reverse();
    for index in indexes_to_delete {
      self.branches.remove(index);
    }
    if self.selected_index >= self.branches.len() {
      self.selected_index = self.branches.len() - 1
    } else if self.selected_index != 0 {
      self.selected_index -= 1
    }
    Ok(())
  }

  async fn create_branch(&mut self, name: String) -> Result<(), Error> {
    let branch = GitBranch { name: name.clone(), is_head: false, upstream: None };
    git_create_branch(&branch).await?;
    self.branches.push(BranchItem::new(branch, true));
    self.branches.sort_by(|a, b| a.branch.name.cmp(&b.branch.name));
    git_checkout_branch_from_name(&name).await?;
    for existing_branch in self.branches.iter_mut() {
      existing_branch.branch.is_head = existing_branch.branch.name == name;
    }
    self.selected_index = self.branches.iter().position(|b| b.branch.name == name).unwrap_or(0);
    Ok(())
  }

  async fn maybe_handle_git_error(&mut self, future: impl Future<Output = Result<(), Error>>) {
    let res = future.await;
    if res.is_err() {
      let error = res.err().unwrap();
      error!("{}", error);
      self.error = Some(error.to_string());
    }
  }

  fn render_list(&mut self, f: &mut Frame<'_>, area: Rect) {
    // TODO don't clone, figure out the index to place the pseudo branch in the list
    let mut branches = self.branches.clone();
    let input_state = self.branch_input.input_state.clone();
    if input_state.value.is_some() && self.mode == Mode::Input {
      let content = input_state.value.unwrap();
      branches.push(BranchItem {
        branch: GitBranch::new(content.clone()),
        staged_for_creation: true,
        staged_for_deletion: false,
        is_valid_name: self.branch_input.input_state.is_valid.unwrap_or(false),
      });
      branches.sort_by(|a, b| a.branch.name.cmp(&b.branch.name));
      self.list_state.select(branches.iter().position(|bi| bi.staged_for_creation))
    } else {
      self.list_state.select(Some(self.selected_index));
    }

    let render_items: Vec<ListItem> = branches.iter().map(|git_branch| git_branch.render()).collect();
    let list = List::new(render_items)
      .block(Block::default().title("Local Branches").borders(Borders::ALL))
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);

    f.render_stateful_widget(list, area, &mut self.list_state);
  }

  fn render_error(&mut self, f: &mut Frame<'_>, area: Rect) {
    if self.error.is_none() {
      return;
    }
    let error_message = self.error.as_ref().unwrap().clone();
    let text = Text::from(error_message);
    let component = Paragraph::new(text)
      .block(Block::bordered().title("Error"))
      .style(Style::from(Color::Red))
      .wrap(Wrap { trim: true });
    f.render_widget(component, area);
  }
}

impl Component for BranchList {
  fn handle_key_events(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    self.clear_error();

    if self.mode == Mode::Input {
      return Ok(Some(Action::UpdateNewBranchName(key)));
    }
    match key {
      KeyEvent { code: KeyCode::Down, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Ok(Some(Action::SelectNextBranch))
      },
      KeyEvent { code: KeyCode::Up, modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Ok(Some(Action::SelectPreviousBranch))
      },
      KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::SHIFT, kind: _, state: _ } => {
        Ok(Some(Action::InitNewBranch))
      },
      KeyEvent { code: KeyCode::Char('c' | 'C'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        Ok(Some(Action::CheckoutSelectedBranch))
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::SHIFT, kind: _, state: _ } => {
        Ok(Some(Action::UnstageBranchForDeletion))
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::CONTROL, kind: _, state: _ } => {
        Ok(Some(Action::DeleteStagedBranches))
      },
      KeyEvent { code: KeyCode::Char('d' | 'D'), modifiers: KeyModifiers::NONE, kind: _, state: _ } => {
        if self.get_selected_branch().is_none() {
          return Ok(None);
        }
        let selected = self.get_selected_branch().unwrap();
        if selected.staged_for_deletion {
          return Ok(Some(Action::DeleteBranch));
        }
        Ok(Some(Action::StageBranchForDeletion))
      },
      _ => Ok(None),
    }
  }

  fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
    match action {
      Action::SelectPreviousBranch => {
        self.select_previous();
        Ok(None)
      },
      Action::SelectNextBranch => {
        self.select_next();
        Ok(None)
      },
      Action::InitNewBranch => {
        self.mode = Mode::Input;
        self.branch_input.init_style();
        Ok(Some(Action::StartInputMode))
      },
      Action::EndInputMod => {
        self.mode = Mode::Selection;
        Ok(None)
      },
      Action::UpdateNewBranchName(key_event) => {
        Ok(
          self
            .branch_input
            .handle_key_event(key_event, self.branches.iter().map(|branch_item| &branch_item.branch).collect()),
        )
      },
      Action::CheckoutSelectedBranch => {
        let result = self.checkout_selected();
        let _ = self.maybe_handle_git_error(result);
        Ok(None)
      },
      Action::CreateBranch(name) => {
        self.mode = Mode::Selection;
        let result = self.create_branch(name);
        let _ = self.maybe_handle_git_error(result);
        Ok(Some(Action::EndInputMod))
      },
      Action::StageBranchForDeletion => {
        self.stage_selected_for_deletion(true);
        Ok(None)
      },
      Action::UnstageBranchForDeletion => {
        self.stage_selected_for_deletion(false);
        Ok(None)
      },
      Action::DeleteBranch => {
        let result = self.deleted_selected();
        let _ = self.maybe_handle_git_error(result);
        Ok(None)
      },
      Action::DeleteStagedBranches => {
        let result = self.delete_staged_branches();
        let _ = self.maybe_handle_git_error(result);
        Ok(None)
      },
      _ => Ok(None),
    }
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
    if self.mode == Mode::Input {
      let layout =
        Layout::new(Direction::Vertical, [Constraint::Fill(1), Constraint::Length(3), Constraint::Length(1)])
          .margin(1)
          .split(area);
      self.render_list(f, layout[0]);
      self.branch_input.render(f, layout[1]);
      self.instruction_footer.render(f, layout[2], &self.branches, self.get_selected_branch());
      return Ok(());
    }

    if self.error.is_some() {
      let err_size = self.error.clone().unwrap().trim().lines().count() + 2;
      let layout = Layout::new(Direction::Vertical, [
        Constraint::Fill(1),
        Constraint::Length(u16::try_from(err_size)?),
        Constraint::Length(1),
      ])
      .margin(1)
      .split(area);
      self.render_list(f, layout[0]);
      self.render_error(f, layout[1]);
      self.instruction_footer.render(f, layout[2], &self.branches, self.get_selected_branch());
      return Ok(());
    }

    let layout = Layout::new(Direction::Vertical, [Constraint::Fill(1), Constraint::Length(1)]).margin(1).split(area);
    self.render_list(f, layout[0]);
    self.instruction_footer.render(f, layout[1], &self.branches, self.get_selected_branch());

    Ok(())
  }
}
