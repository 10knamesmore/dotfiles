<template>
  <div class="container">
    <MyComponent :title="message" @click="handleClick" />
    <ul v-for="item in items" :key="item.id">
      <li>{{ item.name }}</li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import type { PropType } from 'vue'
import MyComponent from './MyComponent.vue'

interface User {
  id: number
  name: string
  email: string
  isActive: boolean
}

const message = ref<string>('Hello World')
const count = ref(42)
const PI = 3.14159

const users: User[] = [
  { id: 1, name: 'Alice', email: 'a@b.com', isActive: true },
]

const activeUsers = computed(() => {
  return users.filter((user: User) => user.isActive)
})

function handleClick(event: MouseEvent): void {
  if (event.ctrlKey) {
    console.log(`Count: ${count.value}`)
    return
  }
  for (const user of users) {
    if (user.isActive && user.name !== 'test') {
      message.value = user.name
    }
  }
}

onMounted(() => {
  console.log('mounted', PI)
})
</script>

<style scoped>
.container {
  display: flex;
  color: #cdd6f4;
}
</style>
