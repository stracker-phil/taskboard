//! The system's Quick Look panel, in-process, showing one local file (ADR 0035). Main thread only.

use std::cell::RefCell;
use std::path::Path;

use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{NSInteger, NSString, NSURL};
use objc2_quick_look_ui::{QLPreviewItem, QLPreviewPanel, QLPreviewPanelDataSource};

use crate::error::{AppError, AppResult};

struct Item {
    url: Retained<NSURL>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Item]
    struct OneFile;

    unsafe impl NSObjectProtocol for OneFile {}

    unsafe impl QLPreviewPanelDataSource for OneFile {
        #[unsafe(method(numberOfPreviewItemsInPreviewPanel:))]
        fn number_of_items(&self, _panel: Option<&QLPreviewPanel>) -> NSInteger {
            1
        }

        #[unsafe(method_id(previewPanel:previewItemAtIndex:))]
        fn item(
            &self,
            _panel: Option<&QLPreviewPanel>,
            _index: NSInteger,
        ) -> Option<Retained<ProtocolObject<dyn QLPreviewItem>>> {
            Some(ProtocolObject::from_retained(self.ivars().url.clone()))
        }
    }
);

thread_local! {
    /// The panel holds its data source unretained.
    static SHOWN: RefCell<Option<Retained<OneFile>>> = const { RefCell::new(None) };
}

/// Opens the panel on `path`, or switches it to `path` when it is already open.
pub fn show(path: &Path) -> AppResult<()> {
    let mtm = MainThreadMarker::new()
        .ok_or_else(|| AppError::Internal("Quick Look runs on the main thread only".into()))?;
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    let source = OneFile::alloc(mtm).set_ivars(Item { url });
    let source: Retained<OneFile> = unsafe { msg_send![super(source), init] };
    let panel = unsafe { QLPreviewPanel::sharedPreviewPanel(mtm) }
        .ok_or_else(|| AppError::Internal("Quick Look isn't available".into()))?;
    unsafe {
        panel.setDataSource(Some(ProtocolObject::from_ref(&*source)));
        panel.reloadData();
    }
    panel.makeKeyAndOrderFront(None);
    SHOWN.with(|shown| *shown.borrow_mut() = Some(source));
    Ok(())
}
