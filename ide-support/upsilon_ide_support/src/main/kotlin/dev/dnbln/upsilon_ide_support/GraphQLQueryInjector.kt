package dev.dnbln.upsilon_ide_support

import com.intellij.lang.injection.MultiHostInjector
import com.intellij.lang.injection.MultiHostRegistrar
import com.intellij.lang.jsgraphql.GraphQLLanguage
import com.intellij.psi.PsiElement
import com.intellij.psi.util.parentOfType
import org.rust.lang.core.psi.*
import org.rust.lang.core.psi.ext.qualifiedName

class GraphQLQueryInjector : MultiHostInjector {
    override fun getLanguagesToInject(registrar: MultiHostRegistrar, context: PsiElement) {
        val q = context as? RsLitExpr ?: return
        val call = q.parentOfType<RsMethodCall>() ?: return

        if (call.valueArgumentList.exprList.firstOrNull() != q) {
            return
        }

        val resolved = call.reference.resolve() ?: return
        val resolvedFn = resolved as? RsFunction ?: return

        val qualifiedName = resolvedFn.qualifiedName

        if (qualifiedName !in setOf(
                "upsilon_test_support::client::gql_query",
                "upsilon_test_support::client::gql_query_with_variables",
                "upsilon_debug_data_driver::client::gql_query_with_variables",
                "upsilon_debug_data_driver::client::gql_query",
                "upsilon_debug_data_driver::client::gql_mutation_with_variables",
                "upsilon_debug_data_driver::client::gql_mutation",
            )
        )
            return

        val range = (q.kind as? RsLiteralKind.String)?.offsets?.value ?: return

        registrar.startInjecting(GraphQLLanguage.INSTANCE)
            .addPlace(null, null, q, range)
            .doneInjecting()
    }

    override fun elementsToInjectIn(): MutableList<out Class<out PsiElement>> = mutableListOf(RsLitExpr::class.java)
}