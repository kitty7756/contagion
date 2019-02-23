use crate::core::vector::*;

#[derive(Clone, PartialEq)]
pub enum GuiType {
    Selected, // Bottom Middle
    SelectionDrag, // Bottom Right
    Score,    // Top Left
    Timer,    // Top Middle
    Rect,
    Menu {
        _window_gui: Box<Gui>,
        _buttons_gui: Box<Vec<Gui>>,
    },
    Button,
}

pub struct Component {
    pub components: Vec<Gui>,
}

impl Component {
    pub fn init_demo() -> Component {
        let selected_ui = Gui::new(GuiType::Selected, 0.1, 0.1, Vector2{x: -0.9, y: -0.9});
        let drag_ui = Gui::new(GuiType::SelectionDrag, 0.0, 0.0, Vector2{x: 0.0, y: 0.0});
        let box_ui = Gui::new(GuiType::Button, 0.3, 0.5, Vector2{x: 0.0, y: 0.0});
        let button1 = Gui::new(GuiType::Button, 0.2, 0.05, Vector2{x: 0.0, y: 0.0});
        let menu_ui = Gui::new(GuiType::Menu{ _window_gui: Box::new(box_ui), _buttons_gui: Box::new(vec!(button1))}, 0.1, 0.125, Vector2{x: -0.9, y: 0.9});

        Component {
            components: vec![selected_ui, drag_ui, menu_ui],
        }
    }
}

#[derive(Clone,PartialEq)]
pub struct Gui {
    pub id: GuiType,
    pub top_left: Vector2,
    pub top_right: Vector2,
    pub bot_left: Vector2,
    pub bot_right: Vector2,
}

// using viewport coordinate [-1,1]
impl Gui {
    // instantiates GUI
    pub fn new(_id: GuiType, w: f64, h: f64, pos: Vector2) -> Gui {
        let _x = w/2.0;
        let _y = h/2.0;
        Gui {
            id: _id,
            top_left: Vector2{x: -_x + pos.x, y: _y + pos.y},
            top_right: Vector2{x: _x + pos.x, y: _y + pos.y},
            bot_left: Vector2{x: -_x + pos.x, y: -_y + pos.y},
            bot_right: Vector2{x: _x + pos.x, y: -_y + pos.y},
        }
    }

    // move position of the GUI
    pub fn move_pos(&mut self, vec: Vector2) {
        self.top_left.x += vec.x;
        self.top_right.x += vec.x;
        self.bot_left.x += vec.x;
        self.bot_right.x += vec.x;
        self.top_left.y += vec.y;
        self.top_right.y += vec.y;
        self.bot_left.y += vec.y;
        self.bot_right.y += vec.y;
    }

    // get dimension of the user interface
    // ordered top_left, top_right, bot_left, bot_right
    pub fn get_dimension(&self) -> (&Vector2, &Vector2, &Vector2, &Vector2) {
        (&self.top_left, &self.top_right, &self.bot_left, &self.bot_right)
    }

    pub fn set_dimension(&mut self, tl: Vector2, tr: Vector2, bl: Vector2, br: Vector2) {
        self.top_left = tl;
        self.top_right = tr;
        self.bot_left = bl;
        self.bot_right = br;
    }

//
}
