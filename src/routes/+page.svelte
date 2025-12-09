<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let query = $state("");
  let greetMsg = $state("");

  $effect(() => {
      invoke("search", { query }).then((res) => {
        if (query.length === 0) {
          greetMsg = "Search for something...";
          return;
        }
        if (typeof res === "string") {
          greetMsg = res;
        }
      });
  });
</script>

<main data-tauri-drag-region class="container">
  <h1>S<span class="underline">can&nbsp;&nbsp;</span></h1>

  <form class="row">
    <input id="greet-input" placeholder="Search for something..." bind:value={query} />
  </form>
  <p>{greetMsg}</p>
</main>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  /*background-color: #f6f6f6;*/

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;

  border-radius: 30px;
}

.container {
  margin: 0;
  opacity: 0.5;
  padding-top: 10vh;
  display: flex;
  flex-shrink: 1;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.underline {
  text-underline-offset: 0.5rem;
  text-decoration: underline;
}

.row {
  display: flex;
  justify-content: center;
}

h1 {
  text-align: center;
}

input {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

input {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2fAA;
  }

  input{
    color: #ffffff;
    background-color: #0f0f0f98;
  }
}

</style>
