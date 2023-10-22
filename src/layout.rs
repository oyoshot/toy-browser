use crate::css::{Unit, Value};
use crate::style::{Display, StyledNode};

pub use self::BoxType::{AnonymousBlock, BlockNode, InlineNode};

#[derive(Debug, Default, Clone, Copy)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub box_type: BoxType<'a>,
    pub children: Vec<LayoutBox<'a>>,
}

pub enum BoxType<'a> {
    BlockNode(&'a StyledNode<'a>),
    InlineNode(&'a StyledNode<'a>),
    AnonymousBlock,
}

impl<'a> LayoutBox<'a> {
    fn new(box_type: BoxType) -> LayoutBox {
        LayoutBox {
            box_type: box_type,
            dimensions: Default::default(),
            children: Vec::new(),
        }
    }

    fn get_inline_container(&mut self) -> &mut LayoutBox<'a> {
        match self.box_type {
            InlineNode(_) | AnonymousBlock => self,
            BlockNode(_) => {
                match self.children.last() {
                    Some(&LayoutBox {
                        box_type: AnonymousBlock,
                        ..
                    }) => {}
                    _ => self.children.push(LayoutBox::new(AnonymousBlock)),
                }
                self.children.last_mut().unwrap()
            }
        }
    }

    fn layout(&mut self, containing_block: Dimensions) {
        match self.box_type {
            BlockNode(_) => self.layout_block(containing_block),
            InlineNode(_) | AnonymousBlock => {} // TODO
        }
    }

    fn layout_block(&mut self, containing_block: Dimensions) {
        // 子の幅は親の幅に依存することがあるので、
        // 子を並べる前にこのボックスの幅を計算する必要がある
        self.calculate_block_width(containing_block);

        // コンテナ内のボックスの位置を決定
        self.calculate_block_position(containing_block);

        // このボックスの子を再帰的にレイアウトする
        self.layout_block_children();

        // 親の高さは子の高さに依存することがあるので、
        // `calculate_height`は子がレイアウトされた後に呼ばれなければならない
        self.calculate_block_height();
    }

    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();

        let auto = Value::Keyword("auto".to_string());
        let mut width = style.value("width").unwrap_or(auto.clone());

        let zero = Value::Length(0.0, Unit::Px);

        let mut margin_left = style.lookup("margin-left", "margin", &zero);
        let mut margin_right = style.lookup("margin-right", "margin", &zero);

        let border_left = style.lookup("border-left-width", "border-width", &zero);
        let border_right = style.lookup("border-right-width", "border-width", &zero);

        let padding_left = style.lookup("padding-left", "padding", &zero);
        let padding_right = style.lookup("padding-right", "padding", &zero);

        let total = sum([
            &margin_left,
            &margin_right,
            &border_left,
            &border_right,
            &padding_left,
            &padding_right,
            &width,
        ]
        .iter()
        .map(|v| v.to_px()));

        if width != auto && total > containing_block.content.width {
            if margin_left == auto {
                margin_left = Value::Length(0.0, Unit::Px);
            }
            if margin_right == auto {
                margin_right = Value::Length(0.0, Unit::Px);
            }
        }

        // 上記の合計が `containing_block.width` と等しくなるように、使用する値を調整する
        // `match` の各アームは合計幅をちょうど `underflow` だけ増加させる
        // その後、すべての値はpx単位の絶対長になる。
        let underflow = containing_block.content.width - total;

        match (width == auto, margin_left == auto, margin_right == auto) {
            // 値が過剰に制約されている場合は、margin_rightを計算する
            (false, false, false) => {
                margin_right = Value::Length(margin_right.to_px() + underflow, Unit::Px);
            }

            // サイズが1つだけautoの場合、その使用値は等号に従う
            (false, true, false) => {
                margin_left = Value::Length(underflow, Unit::Px);
            }
            (false, false, true) => {
                margin_right = Value::Length(underflow, Unit::Px);
            }

            // widthがautoに設定されている場合、その他のautoの値は0になる
            (true, _, _) => {
                if margin_left == auto {
                    margin_left = Value::Length(0.0, Unit::Px);
                }
                if margin_right == auto {
                    margin_right = Value::Length(0.0, Unit::Px);
                }

                if underflow >= 0.0 {
                    // アンダーフローを埋めるために幅を広げる
                    width = Value::Length(underflow, Unit::Px);
                } else {
                    // 幅をマイナスにはできない
                    // 右マージンを調整する
                    width = Value::Length(0.0, Unit::Px);
                    margin_right = Value::Length(margin_right.to_px() + underflow, Unit::Px);
                }
            }

            // margin-leftとmargin-rightが両方ともautoの場合、使用される値は等しくなる
            (false, true, true) => {
                margin_left = Value::Length(underflow / 2.0, Unit::Px);
                margin_right = Value::Length(underflow / 2.0, Unit::Px);
            }
        }

        let d = &mut self.dimensions;
        d.content.width = width.to_px();

        d.padding.left = padding_left.to_px();
        d.padding.right = padding_right.to_px();

        d.border.left = border_left.to_px();
        d.border.right = border_right.to_px();

        d.margin.left = margin_left.to_px();
        d.margin.right = margin_right.to_px();
    }

    /// ブロックのエッジサイズの計算を終了し、それを含むブロック内に配置する
    ///
    /// http://www.w3.org/TR/CSS2/visudet.html#normal-block
    ///
    /// 垂直マージン/パディング/ボーダー寸法と、`x`, `y` 値を設定する
    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        let style = self.get_style_node();
        let d = &mut self.dimensions;

        // マージン，ボーダー，パディングの初期値
        let zero = Value::Length(0.0, Unit::Px);

        // margin-topまたはmargin-bottomが`auto`の場合、使用される値は0
        d.margin.top = style.lookup("margin-top", "margin", &zero).to_px();
        d.margin.bottom = style.lookup("margin-bottom", "margin", &zero).to_px();

        d.border.top = style
            .lookup("border-top-width", "border-width", &zero)
            .to_px();
        d.border.bottom = style
            .lookup("border-bottom-width", "border-width", &zero)
            .to_px();

        d.padding.top = style.lookup("padding-top", "padding", &zero).to_px();
        d.padding.bottom = style.lookup("padding-bottom", "padding", &zero).to_px();

        d.content.x = containing_block.content.x + d.margin.left + d.border.left + d.padding.left;

        // コンテナ内のすべての前のボックスの下にボックスを配置する
        d.content.y = containing_block.content.height
            + containing_block.content.y
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }

    fn get_style_node(&self) -> &'a StyledNode<'a> {
        match self.box_type {
            BlockNode(node) | InlineNode(node) => node,
            AnonymousBlock => panic!("Anonymous block box has no style node"),
        }
    }

    /// ブロックの子要素をコンテンツ領域内に配置する
    ///
    /// `self.dimensions.height` をコンテンツ全体の高さに設定する
    fn layout_block_children(&mut self) {
        let d = &mut self.dimensions;
        for child in &mut self.children {
            child.layout(*d);
            // 各子が前の子の下にレイアウトされるように高さを増加させる
            d.content.height = d.content.height + child.dimensions.margin_box().height;
        }
    }

    /// オーバーフローが見える通常のフローにおける、ブロックレベルの非置換要素の高さ
    fn calculate_block_height(&mut self) {
        // 高さが明示的な長さに設定されている場合は、その長さを使用する
        // それ以外の場合は、`layout_block_children`で設定された値を保持する
        if let Some(Value::Length(h, Unit::Px)) = self.get_style_node().value("height") {
            self.dimensions.content.height = h;
        }
    }
}

fn build_layout_tree<'a>(style_node: &'a StyledNode<'a>) -> LayoutBox<'a> {
    let mut root = LayoutBox::new(match style_node.display() {
        Display::Block => BlockNode(style_node),
        Display::Inline => InlineNode(style_node),
        Display::None => panic!("Root node has not display none."),
    });
    for child in &style_node.children {
        match child.display() {
            Display::Block => root.children.push(build_layout_tree(child)),
            Display::Inline => root
                .get_inline_container()
                .children
                .push(build_layout_tree(child)),
            Display::None => {}
        }
    }
    root
}

impl Rect {
    pub fn expended_by(self, edge: EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
}

impl Dimensions {
    /// コンテンツ領域にパディング、ボーダー、マージンを加えた領域
    pub fn margin_box(self) -> Rect {
        self.border_box().expended_by(self.margin)
    }
    /// コンテンツ領域にパディングとボーダーを加えた領域
    pub fn border_box(self) -> Rect {
        self.padding_box().expended_by(self.border)
    }
    /// コンテンツ領域とそのパディングによってカバーされる領域
    pub fn padding_box(self) -> Rect {
        self.content.expended_by(self.padding)
    }
}

fn sum<I>(iter: I) -> f32
where
    I: Iterator<Item = f32>,
{
    iter.fold(0., |a, b| a + b)
}
