<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let query = $state("");
  let message = $state("");
  let results = $state<SearchResult[]>([]);
  let selectedResultIndex = $state<number>();

  type SearchResult = {
    id: number,
    distance: number,
    label: string,
    path: string,
  }

  type SerialisedFailedResponse = {
    success: false,
    message: string,
  }

  type SerialisedSuccessResponse = {
    success: true,
    data: string,
  }

  type SerialisedSearchResponse = SerialisedFailedResponse | SerialisedSuccessResponse;

  type SearchFailedResponse = {
    success: false,
    message: string,
  }

  type SearchSuccessResponse = {
    success: true,
    data: SearchResult[],
  }

  type SearchResponse = SearchFailedResponse | SearchSuccessResponse;

  const isSearchResponse = (response: any): response is SerialisedSearchResponse => {
    const hasSuccessKey = Object.hasOwn(response, 'success');
    const hasDataKey = Object.hasOwn(response, 'data');
    const dataIsArray = Array.isArray(JSON.parse(response.data));
    const hasMessageKey = Object.hasOwn(response, 'message');
    const hasRequiredKeys = hasSuccessKey;
    const hasOptionalKeys = (hasDataKey && dataIsArray) || hasMessageKey;
    return hasRequiredKeys && hasOptionalKeys;
  };

  const deserialiseResponse = (response: any): SearchResponse|null => {
    if (!isSearchResponse(response)) {
      console.debug("Invalid response from search endpoint", response);
      return null;
    }

    if (!response.success) {
      return {
        ...response,
      } satisfies SearchFailedResponse
    }

    return {
      ...response,
      data: JSON.parse(response.data)
    } satisfies SearchSuccessResponse
  }

  $effect(() => {
    if (query.length === 0) {
      message = "Search for something...";
      return;
    }
      invoke("search", { query }).then((res) => {
        const response = deserialiseResponse(res);

        if (!response) {
          message = "Something went wrong"
          return;
        }

        if (!response.success) {
          message = response.message;
          return;
        }

        if (response.data.length === 0) {
          message = "No results found";
          return;
        }

        message = `${response.data.length} results found!`;
        results = response.data
      });
  });

  const navigateToNextResult = () => {
    console.debug("navigating to next", {from: selectedResultIndex})
    selectedResultIndex = selectedResultIndex !== undefined ? selectedResultIndex+1 : 0;
  };

  const navigateToPreviousResult = () => {
    console.debug("navigating to previous", {from: selectedResultIndex})
    if (selectedResultIndex === undefined) {
      return;
    }
    if (selectedResultIndex === 0) {
      selectedResultIndex = undefined;
      return;
    }
    selectedResultIndex = selectedResultIndex !== undefined ? selectedResultIndex-1 : 0;
  };

  const handleResultsNavigation = (event: KeyboardEvent) => {
    const searchResultElements = document.getElementById("search-results")?.children;
    console.debug({searchResultElements});
    if (!searchResultElements) {
      return;
    }
    switch (event.key) {
      case "ArrowDown":
        navigateToNextResult();
        break;
      case "ArrowUp":
        navigateToPreviousResult();
        break;
      case "Enter":
        console.debug("Launch file");
        break;
      case "Escape":
        console.debug("resetting result selection")
        selectedResultIndex = undefined;
        break;
    }
  }
</script>

<main data-tauri-drag-region class="container">
  <h1>S<span class="underline">can&nbsp;&nbsp;</span></h1>

  <form class="row">
    <input id="greet-input" placeholder="Search for something..." bind:value={query} onkeyup={handleResultsNavigation} />
  </form>
  <p>{message}</p>
  {#if (results.length > 0)}
    <ul id="search-results">
      {#each results as result, index (`${query}-${result.id}`)}
        <li id={`result-${index}`} class={index === selectedResultIndex ? "selectedResult" : ""}>{result.label} ({result.path}) - ({result.distance})</li>
        {:else}
        <li>No results found</li>
        {/each}
    </ul>
  {/if}
</main>

<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #ffffff;

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

.selectedResult {
  background-color: red;
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
