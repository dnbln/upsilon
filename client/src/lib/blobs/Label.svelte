<!--
  -        Copyright (c) 2023 Dinu Blanovschi
  -
  -    Licensed under the Apache License, Version 2.0 (the "License");
  -    you may not use this file except in compliance with the License.
  -    You may obtain a copy of the License at
  -
  -        https://www.apache.org/licenses/LICENSE-2.0
  -
  -    Unless required by applicable law or agreed to in writing, software
  -    distributed under the License is distributed on an "AS IS" BASIS,
  -    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
  -    See the License for the specific language governing permissions and
  -    limitations under the License.
  -->
<script>
    export let label;

    let re = new RegExp('(#([\\\\da-f]{3}){1,2}|(rgb|hsl)a\\\\((\\\\d{1,3}%?,\\\\s?){3}(1|0?\\\\.\\\\d+)\\\\)|(rgb|hsl)\\\\(\\\\d{1,3}%?(,\\\\s?\\\\d{1,3}%?){2}\\\\))');

    function lighten(hsl) {
        if(re.test(hsl)) hsl = 'hsl(0,0%,0%)';
        let hslArray = hsl.toString().slice(4).slice(0, -1).split(',');
        let h = hslArray[0];
        let s = (parseInt(hslArray[1].slice(0, -1)) + 1) % 100;
        let l = (parseInt(hslArray[2].slice(0, -1)) + 40) % 100;


        return 'hsl(' + h + ',' + s + '%,' + l + '%)';
    }

    function darken(hsl) {
        if (re.test(hsl)) hsl = 'hsl(0,0%,0%)';
        let hslArray = hsl.toString().slice(4).slice(0, -1).split(',');
        let h = hslArray[0];
        let s = Math.max(0, parseInt(hslArray[1].slice(0, -1)) - 40);
        let l = Math.max(0, parseInt(hslArray[2].slice(0, -1)) - 12);

        return 'hsl(' + h + ',' + s + '%,' + l + '%)';
    }
</script>

<div class="label" style="background-color: {darken(label.color)}; border: 1px solid {lighten(label.color)}">
    <span style="color: {lighten(label.color)}">{label.title}</span>
</div>

<style lang="scss">
    .label {
      text-align: center;
      padding: 5px 10px;
      border-radius: 1em;
      font-size: .8rem;

        span {
          color: white;
        }
    }
</style>