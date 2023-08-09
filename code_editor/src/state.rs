use {
    crate::{
        change::{ChangeKind, Drift},
        char::CharExt,
        inlays::{BlockInlay, InlineInlay},
        iter::IteratorExt,
        line::Wrapped,
        selection::Affinity,
        str::StrExt,
        token::TokenKind,
        widgets::BlockWidget,
        Change, Line, Point, Range, Selection, Settings, Text, Token,
    },
    std::{
        cell::RefCell,
        cmp,
        collections::HashMap,
        iter, mem,
        rc::Rc,
        slice::Iter,
        sync::{
            atomic,
            atomic::AtomicUsize,
            mpsc,
            mpsc::{Receiver, Sender},
        },
    },
};

#[derive(Debug)]
pub struct Session {
    id: SessionId,
    settings: Rc<Settings>,
    document: Rc<RefCell<Document>>,
    wrap_column: Option<usize>,
    y: Vec<f64>,
    column_count: Vec<Option<usize>>,
    fold_column: Vec<usize>,
    scale: Vec<f64>,
    wraps: Vec<Vec<usize>>,
    wrap_indent_column: Vec<usize>,
    selections: Vec<Selection>,
    pending_selection_index: Option<usize>,
    change_receiver: Receiver<Change>,
}

impl Session {
    pub fn new(document: Rc<RefCell<Document>>) -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);

        let (change_sender, change_receiver) = mpsc::channel();
        let count = document.borrow().text.as_lines().len();
        let mut session = Self {
            id: SessionId(ID.fetch_add(1, atomic::Ordering::AcqRel)),
            settings: Rc::new(Settings::default()),
            document,
            wrap_column: None,
            y: Vec::new(),
            column_count: (0..count).map(|_| None).collect(),
            fold_column: (0..count).map(|_| 0).collect(),
            scale: (0..count).map(|_| 1.0).collect(),
            wraps: (0..count).map(|_| Vec::new()).collect(),
            wrap_indent_column: (0..count).map(|_| 0).collect(),
            selections: vec![Selection::default()].into(),
            pending_selection_index: None,
            change_receiver,
        };
        session.update_y();
        for index in 0..count {
            session.update_wraps(index);
        }
        session
            .document
            .borrow_mut()
            .change_senders
            .insert(session.id, change_sender);
        session
    }

    pub fn id(&self) -> SessionId {
        self.id
    }

    pub fn width(&self) -> f64 {
        self.lines(0, self.document.borrow().text.as_lines().len(), |lines| {
            let mut width: f64 = 0.0;
            for line in lines {
                width = width.max(line.width());
            }
            width
        })
    }

    pub fn height(&self) -> f64 {
        let index = self.document.borrow().text.as_lines().len() - 1;
        let mut y = self.line(index, |line| line.y() + line.height());
        self.blocks(index, index, |blocks| {
            for block in blocks {
                match block {
                    Block::Line {
                        is_inlay: true,
                        line,
                    } => y += line.height(),
                    Block::Widget(widget) => y += widget.height,
                    _ => unreachable!(),
                }
            }
        });
        y
    }

    pub fn settings(&self) -> &Rc<Settings> {
        &self.settings
    }

    pub fn document(&self) -> &Rc<RefCell<Document>> {
        &self.document
    }

    pub fn wrap_column(&self) -> Option<usize> {
        self.wrap_column
    }

    pub fn find_first_line_ending_after_y(&self, y: f64) -> usize {
        match self
            .y
            .binary_search_by(|current_y| current_y.partial_cmp(&y).unwrap())
        {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        }
    }

    pub fn find_first_line_starting_after_y(&self, y: f64) -> usize {
        match self
            .y
            .binary_search_by(|current_y| current_y.partial_cmp(&y).unwrap())
        {
            Ok(index) => index + 1,
            Err(index) => index,
        }
    }

    pub fn line<T>(&self, index: usize, f: impl FnOnce(Line<'_>) -> T) -> T {
        let document = self.document.borrow();
        f(Line {
            y: self.y.get(index).copied(),
            column_count: self.column_count[index],
            fold_column: self.fold_column[index],
            scale: self.scale[index],
            text: &document.text.as_lines()[index],
            tokens: &document.tokens[index],
            inline_inlays: &document.inline_inlays[index],
            wraps: &self.wraps[index],
            wrap_indent_column: self.wrap_indent_column[index],
        })
    }

    pub fn lines<T>(&self, start: usize, end: usize, f: impl FnOnce(Lines<'_>) -> T) -> T {
        let document = self.document.borrow();
        f(Lines {
            y: self.y[start.min(self.y.len())..end.min(self.y.len())].iter(),
            column_count: self.column_count[start..end].iter(),
            fold_column: self.fold_column[start..end].iter(),
            scale: self.scale[start..end].iter(),
            text: document.text.as_lines()[start..end].iter(),
            tokens: document.tokens[start..end].iter(),
            inline_inlays: document.inline_inlays[start..end].iter(),
            wraps: self.wraps[start..end].iter(),
            wrap_indent_column: self.wrap_indent_column[start..end].iter(),
        })
    }

    pub fn blocks<T>(&self, start: usize, end: usize, f: impl FnOnce(Blocks<'_>) -> T) -> T {
        let document = self.document.borrow();
        let mut block_inlays = document.block_inlays.iter();
        while block_inlays
            .as_slice()
            .first()
            .map_or(false, |&(position, _)| position < start)
        {
            block_inlays.next();
        }
        self.lines(start, end, |lines| {
            f(Blocks {
                lines,
                block_inlays,
                position: start,
            })
        })
    }

    pub fn selections(&self) -> &[Selection] {
        &self.selections
    }

    pub fn set_wrap_column(&mut self, wrap_column: Option<usize>) {
        if self.wrap_column == wrap_column {
            return;
        }
        self.wrap_column = wrap_column;
        let count = self.document.borrow().text.as_lines().len();
        for index in 0..count {
            self.update_wraps(index);
        }
        self.update_y();
    }

    pub fn set_cursor(&mut self, cursor: Point, affinity: Affinity) {
        self.selections.clear();
        self.selections.push(Selection {
            anchor: cursor,
            cursor,
            affinity,
        });
        self.pending_selection_index = Some(0);
    }

    pub fn add_cursor(&mut self, cursor: Point, affinity: Affinity) {
        let selection = Selection {
            anchor: cursor,
            cursor,
            affinity,
        };
        self.pending_selection_index = Some(
            match self.selections.binary_search_by(|selection| {
                if selection.end() <= cursor {
                    return cmp::Ordering::Less;
                }
                if selection.start() >= cursor {
                    return cmp::Ordering::Greater;
                }
                cmp::Ordering::Equal
            }) {
                Ok(index) => {
                    self.selections[index] = selection;
                    index
                }
                Err(index) => {
                    self.selections.insert(index, selection);
                    index
                }
            },
        );
    }

    pub fn move_to(&mut self, cursor: Point, affinity: Affinity) {
        let mut pending_selection_index = self.pending_selection_index.unwrap();
        self.selections[pending_selection_index] = Selection {
            cursor,
            affinity,
            ..self.selections[pending_selection_index]
        };
        while pending_selection_index > 0 {
            let prev_selection_index = pending_selection_index - 1;
            if !self.selections[prev_selection_index]
                .should_merge(self.selections[pending_selection_index])
            {
                break;
            }
            self.selections.remove(prev_selection_index);
            pending_selection_index -= 1;
        }
        while pending_selection_index + 1 < self.selections.len() {
            let next_selection_index = pending_selection_index + 1;
            if !self.selections[pending_selection_index]
                .should_merge(self.selections[next_selection_index])
            {
                break;
            }
            self.selections.remove(next_selection_index);
        }
        self.pending_selection_index = Some(pending_selection_index);
    }

    pub fn insert(&mut self, text: Text) {
        let mut changes = Vec::new();
        self.document.borrow_mut().edit(&self.selections, &mut changes, |_, _| {
            (Some(text.clone()), None)
        });
        for change in &changes {
            self.document.borrow_mut().apply_change(change);
        }
    }

    pub fn delete(&mut self) {
        let mut changes = Vec::new();
        self.document.borrow_mut().edit(&self.selections, &mut changes, |_, _| {
            (None, None)
        });
        for change in &changes {
            self.document.borrow_mut().apply_change(change);
        }
    }

    fn update_y(&mut self) {
        let start = self.y.len();
        let end = self.document.borrow().text.as_lines().len();
        if start == end + 1 {
            return;
        }
        let mut y = if start == 0 {
            0.0
        } else {
            self.line(start - 1, |line| line.y() + line.height())
        };
        let mut ys = mem::take(&mut self.y);
        self.blocks(start, end, |blocks| {
            for block in blocks {
                match block {
                    Block::Line { is_inlay, line } => {
                        if !is_inlay {
                            ys.push(y);
                        }
                        y += line.height();
                    }
                    Block::Widget(widget) => {
                        y += widget.height;
                    }
                }
            }
        });
        ys.push(y);
        self.y = ys;
    }

    pub fn handle_changes(&mut self) {
        while let Ok(change) = self.change_receiver.try_recv() {
            for selection in &mut self.selections {
                *selection = selection.apply_change(&change);
            }
        }
    }

    fn update_column_count(&mut self, index: usize) {
        let mut column_count = 0;
        let mut column = 0;
        self.line(index, |line| {
            for wrapped in line.wrappeds() {
                match wrapped {
                    Wrapped::Text { text, .. } => {
                        column += text
                            .chars()
                            .map(|char| char.column_count(self.settings.tab_column_count))
                            .sum::<usize>();
                    }
                    Wrapped::Widget(widget) => {
                        column += widget.column_count;
                    }
                    Wrapped::Wrap => {
                        column_count = column_count.max(column);
                        column = line.wrap_indent_column();
                    }
                }
            }
        });
        self.column_count[index] = Some(column_count.max(column));
    }

    fn update_wraps(&mut self, index: usize) {
        let (wraps, wrap_indent_column) = match self.wrap_column {
            Some(wrap_column) => self.line(index, |line| {
                line.compute_wraps(wrap_column, self.settings.tab_column_count)
            }),
            None => (Vec::new(), 0),
        };
        self.wraps[index] = wraps;
        self.wrap_indent_column[index] = wrap_indent_column;
        self.y.truncate(index + 1);
        self.update_column_count(index);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.document.borrow_mut().change_senders.remove(&self.id);
    }
}

#[derive(Clone, Debug)]
pub struct Lines<'a> {
    pub y: Iter<'a, f64>,
    pub column_count: Iter<'a, Option<usize>>,
    pub fold_column: Iter<'a, usize>,
    pub scale: Iter<'a, f64>,
    pub wrap_indent_column: Iter<'a, usize>,
    pub text: Iter<'a, String>,
    pub tokens: Iter<'a, Vec<Token>>,
    pub inline_inlays: Iter<'a, Vec<(usize, InlineInlay)>>,
    pub wraps: Iter<'a, Vec<usize>>,
}

impl<'a> Iterator for Lines<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let text = self.text.next()?;
        Some(Line {
            y: self.y.next().copied(),
            column_count: *self.column_count.next().unwrap(),
            fold_column: *self.fold_column.next().unwrap(),
            scale: *self.scale.next().unwrap(),
            text,
            tokens: self.tokens.next().unwrap(),
            inline_inlays: self.inline_inlays.next().unwrap(),
            wraps: self.wraps.next().unwrap(),
            wrap_indent_column: *self.wrap_indent_column.next().unwrap(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    lines: Lines<'a>,
    block_inlays: Iter<'a, (usize, BlockInlay)>,
    position: usize,
}

impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self
            .block_inlays
            .as_slice()
            .first()
            .map_or(false, |&(line, _)| line == self.position)
        {
            let (_, block_inlay) = self.block_inlays.next().unwrap();
            return Some(match *block_inlay {
                BlockInlay::Widget(widget) => Block::Widget(widget),
            });
        }
        let line = self.lines.next()?;
        self.position += 1;
        Some(Block::Line {
            is_inlay: false,
            line,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Block<'a> {
    Line { is_inlay: bool, line: Line<'a> },
    Widget(BlockWidget),
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(usize);

#[derive(Debug)]
pub struct Document {
    text: Text,
    tokens: Vec<Vec<Token>>,
    inline_inlays: Vec<Vec<(usize, InlineInlay)>>,
    block_inlays: Vec<(usize, BlockInlay)>,
    change_senders: HashMap<SessionId, Sender<Change>>,
}

impl Document {
    pub fn new(text: Text) -> Self {
        let count = text.as_lines().len();
        let tokens: Vec<_> = (0..count)
            .map(|index| tokenize(&text.as_lines()[index]).collect::<Vec<_>>())
            .collect();
        Self {
            text,
            tokens,
            inline_inlays: (0..count)
                .map(|index| {
                    if index % 2 == 0 {
                        [
                            (20, InlineInlay::Text("XXX".into())),
                            (40, InlineInlay::Text("XXX".into())),
                            (60, InlineInlay::Text("XXX".into())),
                            (80, InlineInlay::Text("XXX".into())),
                        ]
                        .into()
                    } else {
                        Vec::new()
                    }
                })
                .collect(),
            block_inlays: Vec::new(),
            change_senders: HashMap::new(),
        }
    }

    pub fn text(&self) -> &Text {
        &self.text
    }

    fn edit(
        &mut self,
        selections: &[Selection],
        changes: &mut Vec<Change>,
        mut f: impl FnMut(&Text, Point) -> (Option<Text>, Option<Text>),
    ) {
        let mut point = Point::zero();
        let mut prev_range_end = Point::zero();
        for range in selections
            .iter()
            .copied()
            .merge(
                |selection_0, selection_1| match selection_0.merge(selection_1) {
                    Some(selection) => Ok(selection),
                    None => Err((selection_0, selection_1)),
                },
            )
            .map(|selection| selection.range())
        {
            point += range.start() - prev_range_end;
            if !range.is_empty() {
                let change = Change {
                    drift: Drift::Before,
                    kind: ChangeKind::Delete(Range::from_start_and_extent(point, range.extent())),
                };
                self.text.apply_change(change.clone());
                changes.push(change);
            }
            let (insert_text_before, insert_text_after) = f(&self.text, point);
            if let Some(insert_text_before) = insert_text_before {
                let extent = insert_text_before.extent();
                let change = Change {
                    drift: Drift::Before,
                    kind: ChangeKind::Insert(point, insert_text_before),
                };
                point += extent;
                self.text.apply_change(change.clone());
                changes.push(change);
            }
            if let Some(insert_text_after) = insert_text_after {
                let extent = insert_text_after.extent();
                let change = Change {
                    drift: Drift::After,
                    kind: ChangeKind::Insert(point, insert_text_after),
                };
                point += extent;
                self.text.apply_change(change.clone());
                changes.push(change);
            }
            prev_range_end = range.end();
        }
    }

    fn apply_change(&mut self, change: &Change) {
        self.apply_change_to_tokens(change);
        self.apply_change_to_inline_inlays(change);
        for change_sender in self.change_senders.values() {
            change_sender.send(change.clone()).unwrap();
        }
    }

    fn apply_change_to_tokens(&mut self, change: &Change) {
        match change.kind {
            ChangeKind::Insert(point, ref text) => {
                let mut byte = 0;
                let mut index = self.tokens[point.line]
                    .iter()
                    .position(|token| {
                        if byte + token.len > point.byte {
                            return true;
                        }
                        byte += token.len;
                        false
                    })
                    .unwrap_or(self.tokens[point.line].len());
                if byte != point.byte {
                    let token = self.tokens[point.line][index];
                    let mid = point.byte - byte;
                    self.tokens[point.line][index] = Token {
                        len: mid,
                        kind: token.kind,
                    };
                    index += 1;
                    self.tokens[point.line].insert(
                        index,
                        Token {
                            len: token.len - mid,
                            kind: token.kind,
                        },
                    );
                }
                if text.extent().line_count == 0 {
                    self.tokens[point.line]
                        .splice(index..index, tokenize(text.as_lines().first().unwrap()));
                } else {
                    let mut tokens = (point.line..point.line + text.as_lines().len())
                        .map(|line| tokenize(&text.as_lines()[line]).collect::<Vec<_>>())
                        .collect::<Vec<_>>();
                    tokens
                        .first_mut()
                        .unwrap()
                        .splice(..0, self.tokens[point.line][..index].iter().copied());
                    tokens
                        .last_mut()
                        .unwrap()
                        .splice(..0, self.tokens[point.line][index..].iter().copied());
                    self.tokens.splice(point.line..point.line + 1, tokens);
                }
            }
            ChangeKind::Delete(range) => {
                let mut byte = 0;
                let mut start = self.tokens[range.start().line]
                    .iter()
                    .position(|token| {
                        if byte + token.len > range.start().byte {
                            return true;
                        }
                        byte += token.len;
                        false
                    })
                    .unwrap_or(self.tokens[range.start().line].len());
                if byte != range.start().byte {
                    let token = self.tokens[range.start().line][start];
                    let mid = range.start().byte - byte;
                    self.tokens[range.start().line][start] = Token {
                        len: mid,
                        kind: token.kind,
                    };
                    start += 1;
                    self.tokens[range.start().line].insert(
                        start,
                        Token {
                            len: token.len - mid,
                            kind: token.kind,
                        },
                    );
                }
                let mut byte = 0;
                let mut end = self.tokens[range.end().line]
                    .iter()
                    .position(|token| {
                        if byte + token.len > range.end().byte {
                            return true;
                        }
                        byte += token.len;
                        false
                    })
                    .unwrap_or(self.tokens[range.end().line].len());
                if byte != range.end().byte {
                    let token = self.tokens[range.end().line][end];
                    let mid = range.end().byte - byte;
                    self.tokens[range.end().line][end] = Token {
                        len: mid,
                        kind: token.kind,
                    };
                    end += 1;
                    self.tokens[range.end().line].insert(
                        end,
                        Token {
                            len: token.len - mid,
                            kind: token.kind,
                        },
                    );
                }
                if range.start().line == range.end().line {
                    self.tokens[range.start().line].drain(start..end);
                } else {
                    let mut tokens = self.tokens[range.start().line][..start]
                        .iter()
                        .copied()
                        .collect::<Vec<_>>();
                    tokens.extend(self.tokens[range.end().line][end..].iter().copied());
                    self.tokens
                        .splice(range.start().line..range.end().line + 1, iter::once(tokens));
                }
            }
        }
    }

    fn apply_change_to_inline_inlays(&mut self, change: &Change) {
        match change.kind {
            ChangeKind::Insert(point, ref text) => {
                let index = self.inline_inlays[point.line]
                    .iter()
                    .position(|(byte, _)| match byte.cmp(&point.byte) {
                        cmp::Ordering::Less => false,
                        cmp::Ordering::Equal => match change.drift {
                            Drift::Before => true,
                            Drift::After => false,
                        },
                        cmp::Ordering::Greater => true,
                    })
                    .unwrap_or(self.inline_inlays[point.line].len());
                if self.text.extent().line_count == 0 {
                    for (byte, _) in &mut self.inline_inlays[point.line][index..] {
                        *byte += text.extent().byte_count;
                    }
                } else {
                    let mut inline_inlays = (0..text.as_lines().len())
                        .map(|_| Vec::new())
                        .collect::<Vec<_>>();
                    inline_inlays
                        .first_mut()
                        .unwrap()
                        .splice(..0, self.inline_inlays[point.line].drain(..index));
                    inline_inlays.last_mut().unwrap().splice(
                        ..0,
                        self.inline_inlays[point.line]
                            .drain(..)
                            .map(|(byte, inline_inlay)| {
                                (byte + text.extent().byte_count, inline_inlay)
                            }),
                    );
                    self.inline_inlays
                        .splice(point.line..point.line + 1, inline_inlays);
                }
            }
            ChangeKind::Delete(range) => {
                let start = self.inline_inlays[range.start().line]
                    .iter()
                    .position(|&(byte, _)| byte >= range.start().byte)
                    .unwrap_or(self.inline_inlays[range.start().line].len());
                let end = self.inline_inlays[range.end().line]
                    .iter()
                    .position(|&(byte, _)| byte >= range.end().byte)
                    .unwrap_or(self.inline_inlays[range.end().line].len());
                if range.start().line == range.end().line {
                    self.inline_inlays[range.start().line].drain(start..end);
                    for (byte, _) in &mut self.inline_inlays[range.start().line][start..] {
                        *byte = range.start().byte + (*byte - range.end().byte.min(*byte));
                    }
                } else {
                    let mut inline_inlays = self.inline_inlays[range.start().line]
                        .drain(..start)
                        .collect::<Vec<_>>();
                    inline_inlays.extend(self.inline_inlays[range.end().line].drain(end..).map(
                        |(byte, inline_inlay)| {
                            (
                                range.start().byte + byte - range.end().byte.min(byte),
                                inline_inlay,
                            )
                        },
                    ));
                    self.inline_inlays.splice(
                        range.start().line..range.end().line + 1,
                        iter::once(inline_inlays),
                    );
                }
            }
        }
    }
}

fn tokenize(text: &str) -> impl Iterator<Item = Token> + '_ {
    text.split_whitespace_boundaries().map(|string| Token {
        len: string.len(),
        kind: if string.chars().next().unwrap().is_whitespace() {
            TokenKind::Whitespace
        } else {
            TokenKind::Unknown
        },
    })
}
