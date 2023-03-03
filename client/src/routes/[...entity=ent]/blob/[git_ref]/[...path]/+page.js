/*
 *        Copyright (c) 2023 Dinu Blanovschi
 *
 *    Licensed under the Apache License, Version 2.0 (the "License");
 *    you may not use this file except in compliance with the License.
 *    You may obtain a copy of the License at
 *
 *        https://www.apache.org/licenses/LICENSE-2.0
 *
 *    Unless required by applicable law or agreed to in writing, software
 *    distributed under the License is distributed on an "AS IS" BASIS,
 *    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *    See the License for the specific language governing permissions and
 *    limitations under the License.
 */

import {graphql} from '$houdini';

export const _houdini_load = graphql`
  query RepoBlobViewPage($entity: String!, $git_ref: String!, $path: String!) {
    viewer {
      ...NavBar_viewer
      displayName
    }

    repo: lookupRepo(path: $entity) {
      id
      name
      path
      git {
        revspec(revspec: $git_ref) {
          commitFrom {
            sha
            message
            author {
              name
              email
              user {
                id
                username
              }
            }
            committer {
              name
              email
              user {
                id
                username
              }
            }
            tree {
              entries(wholeTree:true) {
                name
              }
            }
            fileContents: blobString(path: $path)
          }
        }
      }
    }
  }
`;

/* @type { import('./$houdini').RepoTreeViewPageVariables } */
export const _RepoBlobViewPageVariables = ({params}) => {
    return {
        entity: params.entity,
        git_ref: params.git_ref,
        path: params.path,
    }
}

/**
 * @param { import('./$houdini').AfterLoadEvent }
 */
export function _houdini_afterLoad({ data, event }) {
    return {
        path: event.params.path,
    }
}