// SPDX-License-Identifier: GPL-3.0-or-later
// Controlled matrix for rowplay-qt#143: the same QListModel backend under four
// QML exposure paths. Only the registration and the QML line that names the
// model differ between variants.
use qtbridge::{QApp, QListModel, QListModelBase, QModelItem, qobject};
#[cfg(any(feature = "singleton", feature = "creatable"))]
use qtbridge::QmlElement;

#[derive(Clone, Default, QModelItem)]
pub struct Row { pub value: i32 }

#[derive(Default)]
pub struct Backend { rows: Vec<Row>, resets: i32 }

impl QListModel for Backend {
    type Item = Row;
    fn len(&self) -> usize { self.rows.len() }
    fn get(&self, index: usize) -> Option<&Self::Item> { self.rows.get(index) }
    fn reset_unnotified(&mut self) {
        self.resets += 1;
        self.rows = vec![Row { value: self.resets }];
    }
}

#[qobject(NoQmlElement, ConvertToCamelCase, Base = QListModel)]
impl Backend {
    #[qslot]
    fn reset_rows(&mut self) {
        eprintln!("rust: reset_rows enter (resets so far = {})", self.resets);
        self.reset();
        eprintln!("rust: reset_rows exit (resets = {}, len = {})", self.resets, self.rows.len());
    }
}

#[cfg(any(feature = "singleton", feature = "creatable"))]
impl QmlElement for Backend {
    const URI: &str = "ResetRepro";
    const ELEMENT_NAME: &str = "Backend";
    const MAJOR_VERSION: u8 = 1;
    const MINOR_VERSION: u8 = 0;
    const IS_SINGLETON: bool = cfg!(feature = "singleton");
}

#[cfg(feature = "singleton")]       const QML: &[u8] = include_bytes!("../qml/singleton.qml");
#[cfg(feature = "creatable")]       const QML: &[u8] = include_bytes!("../qml/creatable.qml");
#[cfg(feature = "initial")]         const QML: &[u8] = include_bytes!("../qml/initial.qml");
#[cfg(feature = "initial_selfref")] const QML: &[u8] = include_bytes!("../qml/initial_selfref.qml");

fn main() {
    let mut app = QApp::new();
    #[cfg(any(feature = "singleton", feature = "creatable"))]
    app.register::<Backend>();
    #[cfg(feature = "initial")]
    app.set_initial_object("backend", std::rc::Rc::new(std::cell::RefCell::new(Backend::default())));
    #[cfg(feature = "initial_selfref")]
    app.set_initial_object("model", std::rc::Rc::new(std::cell::RefCell::new(Backend::default())));
    app.load_qml(QML).run();
    eprintln!("rust: event loop returned");
}
