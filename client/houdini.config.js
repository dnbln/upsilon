/// <references types="houdini-svelte">

const strNewtype = {"type": "string"};

/** @type {import('houdini').ConfigFile} */
const config = {
    "watchSchema": {
        "url": "http://127.0.0.1:8000/graphql"
    },
    "plugins": {
        "houdini-svelte": {
            "static": true,
        }
    },
    "scalars": {
        "OrganizationId": strNewtype,
        "OrganizationName": strNewtype,
        "OrganizationDisplayName": strNewtype,
        "TeamId": strNewtype,
        "TeamName": strNewtype,
        "TeamDisplayName": strNewtype,
        "RepoId": strNewtype,
        "RepoName": strNewtype,
        "UserId": strNewtype,
        "Username": strNewtype,
        "UserDisplayName": strNewtype,
    }
}

export default config
