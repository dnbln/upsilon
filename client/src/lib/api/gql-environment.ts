import {
    Environment,
    Network,
    RecordSource,
    Store,
} from 'relay-runtime';

import type {
    RequestParameters,
    Variables,
} from 'relay-runtime';

const gqlEndpoint = 'http://localhost:8000/graphql';

async function fetchQuery(request: RequestParameters, variables: Variables) {
    const response = await fetch(gqlEndpoint, {
        method: 'POST',
        headers: {
            'content-type': 'application/json',
        },
        body: JSON.stringify({
            query: request.text,
            variables,
        }),
    });

    return response.json();
}

const network = Network.create(fetchQuery);
const store = new Store(new RecordSource());

const environment = new Environment({
    network,
    store,
});

export default environment;