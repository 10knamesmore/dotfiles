; extends

; 给 Vue 顶层块标签单独分组
((tag_name) @tag.template.vue
  (#eq? @tag.template.vue "template")
  (#set! priority 110))

((tag_name) @tag.script.vue
  (#eq? @tag.script.vue "script")
  (#set! priority 110))

((tag_name) @tag.style.vue
  (#eq? @tag.style.vue "style")
  (#set! priority 110))

; 组件标签同时匹配 PascalCase 和 kebab-case
((tag_name) @tag.vue
  (#match? @tag.vue "^[A-Z]")
  (#set! priority 110))

((tag_name) @tag.vue
  (#match? @tag.vue "^[a-z][a-z0-9]*-[a-z0-9-]+$")
  (#set! priority 110))

; 内建 HTML 标签仍按小写单词处理
((tag_name) @tag.builtin.vue
  (#match? @tag.builtin.vue "^[a-z][a-z0-9]*$")
  (#not-any-of? @tag.builtin.vue "template" "script" "style")
  (#set! priority 105))
