use color_eyre::owo_colors::OwoColorize;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span},
  widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::{
  action::Action,
  components::Component,
  error::Error,
  git::repo::{GitBranch, GitRepo},
  tui::Frame,
};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct BranchItem {
  branch: GitBranch,
  marked_for_deletion: bool,
}

impl BranchItem {
  pub fn render(&self) -> ListItem {
    let mut item = ListItem::new(self.branch.name.clone());
    if self.marked_for_deletion {
      item = item.style(Color::Red);
    }
    item
  }

  pub fn toggle_for_deletion(&mut self) {
    self.marked_for_deletion = !self.marked_for_deletion;
  }
}

pub struct GitBranchList {
  repo: GitRepo,
  branches: Vec<BranchItem>,
  state: ListState,
}

impl Default for GitBranchList {
  fn default() -> Self {
    Self::new()
  }
}

impl GitBranchList {
  pub fn new() -> Self {
    let mut repo = GitRepo::from_cwd().unwrap();
    let branches = repo
      .local_branches()
      .unwrap()
      .iter()
      .map(|branch| BranchItem { branch: branch.clone(), marked_for_deletion: false })
      .collect();
    GitBranchList { repo, branches, state: ListState::default().with_selected(Some(0)) }
  }

  pub fn select_previous(&mut self) {
    if self.state.selected().is_none() {
      self.state.select(Some(0));
    }

    let selected = self.state.selected().unwrap();
    let final_index = self.branches.len() - 1;

    if selected == 0 {
      self.state.select(Some(final_index));
      return;
    }
    self.state.select(Some(selected - 1))
  }

  pub fn select_next(&mut self) {
    if self.state.selected().is_none() {
      self.state.select(Some(0));
    }

    let selected = self.state.selected().unwrap();
    let final_index = self.branches.len() - 1;

    if selected == final_index {
      self.state.select(Some(0));
      return;
    }
    self.state.select(Some(selected + 1))
  }

  fn get_selected_branch(&self) -> Option<&BranchItem> {
    let selected_index = self.state.selected()?;
    self.branches.get(selected_index)
  }

  pub fn toggle_selected_for_deletion(&mut self) -> Result<(), Error> {
    if self.state.selected().is_none() {
      return Ok(());
    }
    let selected_index = self.state.selected().unwrap();
    let selected = self.branches.get_mut(selected_index);
    if selected.is_none() {
      return Ok(());
    }
    selected.unwrap().toggle_for_deletion();
    Ok(())
  }

  pub fn deleted_selected(&mut self) -> Result<(), Error> {
    if self.state.selected().is_none() {
      return Ok(());
    }
    let selected_index = self.state.selected().unwrap();
    let selected = self.branches.get(selected_index);
    if selected.is_none() {
      return Ok(());
    }
    let delete_result = self.repo.delete_branch(&selected.unwrap().branch);
    if delete_result.is_err() {
      return Ok(());
    }
    self.branches.remove(selected_index);
    Ok(())
  }
}

impl Component for GitBranchList {
  fn handle_key_events(&mut self, key: KeyEvent) -> color_eyre::Result<Option<Action>> {
    if key.code == KeyCode::Down {
      return Ok(Some(Action::SelectNextBranch));
    }
    if key.code == KeyCode::Up {
      return Ok(Some(Action::SelectPreviousBranch));
    }
    if key.code == KeyCode::Char('d') {
      return Ok(Some(Action::ToggleBranchMarkedForDeletion));
    }
    if key.code == KeyCode::Delete || key.code == KeyCode::Backspace {
      return Ok(Some(Action::DeleteBranch));
    }
    if (key.code == KeyCode::Delete || key.code == KeyCode::Backspace) && key.modifiers == KeyModifiers::SHIFT {
      return Ok(Some(Action::DeleteAllMarkedBranches))
    }
    Ok(None)
  }

  fn update(&mut self, action: Action) -> color_eyre::Result<Option<Action>> {
    if action == Action::SelectNextBranch {
      self.select_next()
    }
    if action == Action::SelectPreviousBranch {
      self.select_previous()
    }
    if action == Action::ToggleBranchMarkedForDeletion {
      self.toggle_selected_for_deletion()?
    }
    if action == Action::DeleteBranch {
      self.deleted_selected()?
    }
    if action == Action::DeleteAllMarkedBranches {
      // TODO
    }
    Ok(None)
  }

  fn draw(&mut self, f: &mut Frame<'_>, area: Rect) -> color_eyre::Result<()> {
    let layout = Layout::new(Direction::Vertical, [Constraint::Fill(1), Constraint::Length(1)]).margin(1).split(area);

    let render_items: Vec<ListItem> = self.branches.iter().map(|git_branch| git_branch.render()).collect();
    let list = List::new(render_items)
      .block(Block::default().title("Local Branches").borders(Borders::ALL))
      .style(Style::default().fg(Color::White))
      .highlight_style(Style::default().add_modifier(Modifier::ITALIC).add_modifier(Modifier::BOLD))
      .highlight_symbol("→")
      .repeat_highlight_symbol(true);

    let mut commands = vec![Span::raw("q: Quit")];
    if self.get_selected_branch().is_some() && self.get_selected_branch().unwrap().marked_for_deletion {
      commands.push(Span::raw(" | d: Unmark for deletion"));
    } else {
      commands.push(Span::raw(" | d: Mark for deletion"));
    }
    commands.push(Span::raw(" | ←: Delete branch"));
    commands.push(Span::raw(" | ⇧ + ←: Delete all"));
    let footer = Line::from(commands);

    f.render_stateful_widget(list, layout[0], &mut self.state);
    f.render_widget(footer, layout[1]);

    Ok(())
  }
}
