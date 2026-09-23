use speech::ffi::sp_final_result_gate_admitted_mask;

const PARTIAL: u8 = 0;
const FINAL: u8 = 1;
const ERROR: u8 = 2;
const EMPTY: u8 = 3;

fn admitted(events: &[u8]) -> u64 {
    unsafe { sp_final_result_gate_admitted_mask(events.as_ptr(), events.len()) }
}

#[test]
fn partial_results_do_not_consume_the_one_shot() {
    assert_eq!(admitted(&[PARTIAL, PARTIAL, FINAL]), 0b100);
    assert_eq!(admitted(&[PARTIAL, FINAL, PARTIAL, FINAL]), 0b0010);
}

#[test]
fn an_error_after_partial_results_completes_the_one_shot() {
    assert_eq!(admitted(&[PARTIAL, ERROR, FINAL]), 0b010);
}

#[test]
fn only_the_first_terminal_event_is_admitted() {
    assert_eq!(admitted(&[FINAL, ERROR, FINAL]), 0b001);
    assert_eq!(admitted(&[ERROR, FINAL, ERROR]), 0b001);
}

#[test]
fn a_callback_without_result_or_error_is_terminal() {
    assert_eq!(admitted(&[PARTIAL, EMPTY, FINAL]), 0b010);
}

#[test]
fn partial_results_alone_never_complete() {
    assert_eq!(admitted(&[PARTIAL; 8]), 0);
    assert_eq!(admitted(&[]), 0);
}
