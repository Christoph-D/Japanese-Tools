<?xml version="1.0"?>
<xsl:stylesheet version="1.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
 xmlns:exslt="http://exslt.org/common"
 xmlns:math="http://exslt.org/math"
 xmlns:date="http://exslt.org/dates-and-times"
 xmlns:func="http://exslt.org/functions"
 xmlns:set="http://exslt.org/sets"
 xmlns:str="http://exslt.org/strings"
 xmlns:dyn="http://exslt.org/dynamic"
 xmlns:saxon="http://icl.com/saxon"
 xmlns:xalanredirect="org.apache.xalan.xslt.extensions.Redirect"
 xmlns:xt="http://www.jclark.com/xt"
 xmlns:libxslt="http://xmlsoft.org/XSLT/namespace"
 xmlns:test="http://xmlsoft.org/XSLT/"
 extension-element-prefixes="exslt math date func set str dyn saxon xalanredirect xt libxslt test"
 exclude-result-prefixes="math str">
<xsl:output omit-xml-declaration="yes" indent="no" encoding="utf-8"/>
<xsl:param name="inputFile">-</xsl:param>

<xsl:template match="/">
  <xsl:call-template name="t1"/>
</xsl:template>

<xsl:template name="translation">
  <xsl:choose>
    <xsl:when test="count(*)=0">
      <xsl:value-of select="."/>
    </xsl:when>
    <xsl:otherwise>
      <xsl:for-each select="*">
        <xsl:choose>
          <xsl:when test="name()='bracket' or name()='birthdeath'">
            <xsl:value-of select="'('"/>
            <xsl:call-template name="translation"/>
            <xsl:value-of select="')'"/>
          </xsl:when>
          <xsl:otherwise>
            <xsl:call-template name="translation"/>
          </xsl:otherwise>
        </xsl:choose>
        <xsl:value-of select="' '"/>
      </xsl:for-each>
    </xsl:otherwise>
  </xsl:choose>
</xsl:template>

<xsl:template name="t1">
  <xsl:for-each select="entries/entry">
    <xsl:for-each select="form/orth">
      <xsl:value-of select="."/>
      <xsl:if test="position()!=last()">
        <xsl:value-of select="'◊'"/>
      </xsl:if>
    </xsl:for-each>
    <xsl:value-of select="'□'"/>
    <xsl:value-of select="form/reading/hira"/>
    <xsl:value-of select="','"/>
    <xsl:value-of select="'□'"/>
    <xsl:for-each select="gramGrp/pos">
      <xsl:value-of select="@type"/>
      <xsl:if test="position()!=last()">
        <xsl:value-of select="','"/>
      </xsl:if>
    </xsl:for-each>
    <xsl:value-of select="'□'"/>
    <xsl:for-each select="sense">
      <xsl:if test="last()>1">
        <xsl:number value="position()" format="1. "/>
      </xsl:if>
      <xsl:for-each select="trans/tr">
        <xsl:call-template name="translation"/>
        <xsl:if test="position()!=last()">
          <xsl:value-of select="', '"/>
        </xsl:if>
      </xsl:for-each>
      <xsl:if test="position()!=last()">
        <xsl:value-of select="' '"/>
      </xsl:if>
    </xsl:for-each>
    <xsl:value-of select="'&#10;'"/>
  </xsl:for-each>
</xsl:template>
</xsl:stylesheet>
