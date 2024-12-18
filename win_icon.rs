use embed_resource::CompilationResult;

fn main() {
  let result = embed_resource::compile("res/win_icon.rc", embed_resource::NONE);
  assert!(result == CompilationResult::Ok || result == CompilationResult::NotWindows);
}
