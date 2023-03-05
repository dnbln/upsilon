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

import { lookupLangmap, lookupHljsLangImpl } from './langMapImpl';

export type Lang = {
	id: string;
	name: string;
	hljs?: string;
	hljs_def?: any;
	icon?: string;
	category?: string;
	parent?: string;
	matcher: (filepath: string, filename: string) => boolean;
	children?: string[];
};

export const getLang = (filepath: string): Lang => lookupLangmap(filepath);

export const lookupHljsLang = (lang: Lang): any => lookupHljsLangImpl(lang);
